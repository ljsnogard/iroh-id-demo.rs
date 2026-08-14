```
iroh 是如何找到第一个中继节点的？
```


是的，这里其实涉及 iroh 里一个很关键的设计：**“我怎么找到对方”与“我怎么找到第一个 Relay”是两个不同的问题。**

对于你前面的 demo，如果使用：

```rust
Endpoint::bind(presets::N0).await?
```

那么 iroh **并不是先通过某个中心服务器询问“离我最近的 Relay 是谁”**。默认 Relay 的地址本身就已经包含在 iroh 的默认配置里。

### 1. 第一个 Relay 从哪里来？

`presets::N0` 会给 Endpoint 配置 n0 的默认 Relay servers。当前 iroh 1.0.3 的源码明确说明，`N0` preset 包含：

* DNS Address Lookup
* n0 提供的默认 Relay servers
* TLS crypto provider

并且 `N0` 最终会设置 `default_relay_mode()`。([Docs.rs][1])

当前默认生产环境的 Relay 列表包括：

```text
use1-1.relay.n0.iroh.link
usw1-1.relay.n0.iroh.link
euc1-1.relay.n0.iroh.link
aps1-1.relay.n0.iroh.link
```

也就是北美东、西、欧洲和亚太几个 Relay。([Docs.rs][2])

所以从机制上说：

```text
你的程序
   │
   │ Endpoint::bind(presets::N0)
   │
   ▼
iroh 默认配置
   │
   ├── use1-1.relay.n0.iroh.link
   ├── usw1-1.relay.n0.iroh.link
   ├── euc1-1.relay.n0.iroh.link
   └── aps1-1.relay.n0.iroh.link
           │
           ▼
       选择/连接 Relay
```

**并不存在一个“第一个 Relay 帮我发现其他 Relay”的 bootstrap Relay。**

这和 BitTorrent tracker、DHT bootstrap node 等机制有明显区别。

---

## 2. 那它怎么知道“最近”的 Relay？

这里要稍微精确一点。

iroh 文档把它描述为 Endpoint 创建后会连接到“closest Relay”，并将其作为自己的 **home relay**。([Docs.rs][3])

但这里的“closest”不要理解成：

> 先访问一个中央服务器，根据你的 IP 算出距离，然后告诉你应该连接哪个 Relay。

实际上默认 Relay 列表已经存在，iroh 可以对这些候选 Relay 进行连接/可用性判断，从而选择合适的 Relay。

所以逻辑更接近：

```text
       已知的 Relay candidates
              │
       ┌──────┼──────┐
       ▼      ▼      ▼
      US     EU      AP
       │      │      │
       └──────┼──────┘
              │
              ▼
        选择 home relay
```

这就是为什么 iroh 可以做到**完全没有预先联系任何 iroh 服务的情况下启动 Endpoint**。

---

## 3. 这其实解决了一个很重要的 Bootstrap 问题

任何 P2P 系统都有一个非常基础的问题：

> 如果我要去连接一个陌生节点，在我甚至还不知道怎么访问网络中的节点之前，我怎么找到第一个节点？

通常叫 **bootstrap problem**。

例如：

```text
DHT
 │
 └── 必须预置 bootstrap nodes

BitTorrent
 │
 └── tracker / DHT bootstrap

Tor
 │
 └── directory / bridge bootstrap

iroh
 │
 └── 默认 Relay URLs + address lookup infrastructure
```

iroh 采用的是一种相当直接的方案：

**把基础设施的入口作为软件默认配置的一部分。**

所以你的程序实际上隐含依赖了 n0 提供的公共基础设施。

---

## 4. 但这里还有第二个“发现”机制

这也是你前面那个 Demo 很值得理解的地方。

假设：

```text
Server EndpointId = S
Client EndpointId = C
```

客户端只知道：

```text
S
```

它并不知道：

```text
Server IP
Server UDP port
Server Relay
```

那么怎么办？

这时候才轮到 **Address Lookup / Discovery**。

你的：

```rust
Endpoint::bind(presets::N0)
```

除了默认 Relay，还安装了：

```text
PkarrPublisher
PkarrResolver
DnsAddressLookup
```

它们使用 n0 的 DNS/Pkarr infrastructure；官方文档明确说明，N0 preset 会配置这些 lookup 服务，而它们可以把 `EndpointId` 解析成实际的 IP 地址和 Relay URLs。([Docs.rs][1])

因此整个过程实际上是：

```text
                    Client
                      │
                      │ 已知 ServerId
                      ▼
              Address Lookup
                      │
                      │ ServerId → EndpointAddr
                      ▼
             ┌────────────────┐
             │ Server address │
             │                │
             │ IP addresses   │
             │ Relay URLs     │
             └───────┬────────┘
                     │
          ┌──────────┴──────────┐
          ▼                     ▼
      direct IP             home Relay
          │                     │
          └──────────┬──────────┘
                     ▼
                  Server
```

官方 FAQ 也明确把这个 distinction 说得很清楚：

> Endpoint ID 本身只能用于身份识别，并不能告诉 iroh 如何寻址；Discovery 才负责把 Endpoint ID 解析成 IP 地址和 Relay URL。([iroh][4])

---

## 5. Relay 和 Address Lookup 是两个不同的东西

这个区分非常重要。

### Address Lookup

解决：

> **“Server ID 对应的 Server 在哪里？”**

例如：

```text
EndpointId
    │
    ▼
Address Lookup
    │
    ▼
IP:port
Relay URL
```

### Relay

解决：

> **“我和 Server 之间怎么通信？”**

例如：

```text
Client ───── Relay ───── Server
```

Relay 还承担 NAT traversal 的辅助作用。

双方先通过 Relay 联系，然后尝试 UDP hole punching 建立：

```text
Client ───────────────── Server
             direct
```

成功后，Relay 通常退出数据路径；如果无法建立 direct connection，则继续通过 Relay 转发。([iroh][5])

---

## 6. 所以你的 Demo 实际上依赖了两套 n0 基础设施

你现在的代码看起来只有：

```rust
Endpoint::bind(presets::N0)
```

但背后实际上发生了不少事情：

```text
                 iroh-id-demo
                       │
                       ▼
               presets::N0
                       │
          ┌────────────┴────────────┐
          │                         │
          ▼                         ▼
   Default Relay Map          Address Lookup
          │                         │
          ▼                         ▼
   n0 Public Relays          n0 DNS / Pkarr
          │                         │
          │                         │
          ▼                         ▼
   NAT traversal /          EndpointId
   encrypted relay          → EndpointAddr
```

所以你的 Demo **并不是完全去中心化的**。

它的通信数据本身可以做到 P2P，而且 Relay 只是 NAT traversal / fallback；但是 **bootstrap 和 endpoint discovery 依赖 n0 提供的公共基础设施**。官方也明确说默认 public relays 和 address lookup 都由 n0.computer 提供。([iroh][6])

---

### 一个特别值得注意的结论

因此，如果你以后想把这个 Demo 做成一个**真正独立部署的 iroh 应用**，你需要考虑的不仅仅是：

> “我自己搭一个 Relay。”

还需要考虑：

```text
                    自建 iroh 网络
                         │
          ┌──────────────┴──────────────┐
          │                             │
      Relay infrastructure        Discovery infrastructure
          │                             │
          ▼                             ▼
    自己的 Relay                 自己的 EndpointId
    server(s)                    → address lookup
```

也就是说，**Relay 和 Discovery 是两个独立的基础设施组件**。

这也是为什么 iroh 官方在生产部署建议中把 dedicated infrastructure 分成 Relay 和 Address Lookup 两部分。([iroh][7])

如果你的最终目标是把这个 demo 继续扩展成你之前设想的那种 **“客户端只需要一个服务器 ID，不依赖 n0 公共服务，我自己控制整个网络”** 的架构，那么下一步其实就很值得研究 iroh 的 `RelayMap`、`RelayMode`、`Discovery` / `AddressLookup` 这几个 API。

[1]: https://docs.rs/iroh/latest/iroh/endpoint/presets/struct.N0.html?utm_source=chatgpt.com "N0 in iroh::endpoint::presets - Rust"
[2]: https://docs.rs/iroh/latest/src/iroh/defaults.rs.html?utm_source=chatgpt.com "defaults.rs - source"
[3]: https://docs.rs/iroh/latest/iroh/index.html?utm_source=chatgpt.com "iroh - Rust"
[4]: https://docs.iroh.computer/about/faq?utm_source=chatgpt.com "FAQ - iroh"
[5]: https://docs.iroh.computer/concepts/relays?utm_source=chatgpt.com "Relays - iroh"
[6]: https://docs.iroh.computer/iroh-services/relays/public?utm_source=chatgpt.com "Public Relays - iroh"
[7]: https://docs.iroh.computer/deployment/dedicated-infrastructure?utm_source=chatgpt.com "Dedicated Infrastructure - iroh"
