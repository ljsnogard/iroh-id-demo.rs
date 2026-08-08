# iroh ID Demo

这是一个非常简单的 [iroh](https://iroh.computer/) 网络通信 Demo。

几乎全部内容，包括这个 README 都是 ChatGPT 生成的，作者运行了一次成功了，写下了这行。

程序将**客户端和服务端功能放在同一个可执行文件中**，通过命令行参数决定当前进程运行客户端还是服务端。

这个 Demo 的主要目的不是实现一个完整的应用协议，而是演示：

* 如何创建 iroh `Endpoint`
* 如何获取 Endpoint 的 iroh Identity / `EndpointId`
* 如何让两个完全独立启动的进程通过 `EndpointId` 建立连接
* 如何使用 ALPN 标识应用层协议
* 如何通过双向 QUIC stream 交换数据
* 如何获得连接对端经过 iroh 身份认证后的 `EndpointId`

## 工作流程

服务端和客户端启动时，都会由 iroh 自动生成一个新的随机 Identity。

由于两个进程启动时彼此并不知道对方的身份，因此需要先启动服务端。

服务端启动后打印自己的 `EndpointId`：

```text
Server started.
Server ID:
<server-endpoint-id>

Waiting for client...
```

然后将这个 ID 复制给客户端：

```text
cargo run -- client <server-endpoint-id>
```

客户端启动时同样会生成自己的 Identity，然后使用服务端的 `EndpointId` 建立连接。

连接建立后，客户端向服务端发送自己的 `EndpointId`，服务端再返回自己的 ID 和客户端发送的 ID。

整体流程如下：

```text
                    iroh network
              ┌─────────────────────┐
              │                     │
              │                     │
┌─────────────┴──────────┐  ┌───────┴────────────────┐
│        Server          │  │         Client          │
│                        │  │                         │
│ generate Identity      │  │ generate Identity       │
│        │               │  │         │               │
│        ▼               │  │         ▼               │
│ Server EndpointId      │  │ Client EndpointId       │
│        │               │  │         │               │
│        │  copy ID      │  │         │               │
│        └───────────────┼─►│ connect(server_id)      │
│                        │  │         │               │
│                        │◄─┼─────────┘               │
│                        │  │                         │
│ receive Client ID      │◄─┤ Client ID               │
│                        │  │                         │
│ Server ID + Client ID  ├─►│                         │
└────────────────────────┘  └─────────────────────────┘
```

## 项目结构

Demo 有意保持非常简单：

```text
iroh-id-demo/
├── Cargo.toml
├── Cargo.lock
└── src/
    └── main.rs
```

所有客户端和服务端逻辑都位于 `main.rs` 中。

## 依赖

主要依赖：

* [iroh](https://crates.io/crates/iroh) —— P2P 网络连接和 Identity
* [tokio](https://crates.io/crates/tokio) —— 异步运行时
* [anyhow](https://crates.io/crates/anyhow) —— 错误处理

当前 Demo 使用：

```toml
iroh = "1.0.3"
```

Rust Edition：

```toml
edition = "2024"
```

## 编译

首先确保已经安装 Rust。

然后：

```bash
cargo build
```

或者直接使用：

```bash
cargo run
```

## 启动服务端

执行：

```bash
cargo run -- server
```

服务端会生成一个新的 iroh Identity，并打印自己的 Endpoint ID：

```text
Server started.
Server ID:
2c7cxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx

Waiting for client...
```

复制这个 ID。

## 启动客户端

打开另一个终端，将刚才复制的 Server ID 作为参数：

```bash
cargo run -- client 2c7cxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

客户端也会生成自己的 Identity：

```text
Client started.
Client ID:
4f81yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy

Connecting to server:
2c7cxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx

Connected.
```

然后客户端发送自己的 ID。

服务端最终会看到：

```text
Client connected.
Remote endpoint: 4f81yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy

Client reported ID:
4f81yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy

Response sent.
Server ID : 2c7cxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
Client ID : 4f81yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy
```

客户端收到：

```text
Server response:
SERVER 2c7cxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
CLIENT 4f81yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy
```

## Identity 与 EndpointId

iroh Endpoint 有自己的密码学 Identity。

在这个 Demo 中：

```rust
let endpoint = ...;
let id = endpoint.id();
```

得到的 `EndpointId` 是这个 Endpoint 的身份标识。

服务端和客户端都在启动时创建新的 Endpoint，因此每次重新启动程序时，默认都会产生新的 Identity。

因此：

```text
第一次启动 Server
    ↓
Server ID = A

关闭 Server

第二次启动 Server
    ↓
Server ID = B
```

通常：

```text
A != B
```

所以客户端不能永久保存本次运行得到的 Server ID 并期待下一次启动仍然有效。

如果需要固定的服务器身份，就应该使用持久化的 iroh `SecretKey`，而不是每次启动时生成新的随机 Identity。

## 为什么客户端需要 Server ID？

这个 Demo 中客户端主动连接服务端，因此客户端必须知道：

```text
Server EndpointId
```

才能指定连接目标。

启动时两个进程互相不知道对方：

```text
Server                           Client

不知道 Client ID                不知道 Server ID
       │                               │
       │                               │
       └────────── 无法主动发现 ────────┘
```

因此 Demo 采用最简单的 Bootstrapping 方法：

```text
1. 启动 Server
2. Server 生成 Identity
3. Server 打印 EndpointId
4. 用户复制 EndpointId
5. 启动 Client，并把 EndpointId 作为参数
6. Client 使用 EndpointId 连接 Server
```

这里的“复制 ID”只是 Demo 使用的人工 Bootstrap 方式。

真实应用通常需要另外的 Bootstrap / Discovery 机制，例如：

* QR Code
* 配置文件
* URL / Deep Link
* Rendezvous Server
* DNS
* Pkarr
* 其他应用层服务发现机制

iroh 本身负责建立 P2P 连接，但“用户第一次如何获得对端身份”仍然是应用需要解决的问题。

## ALPN

Demo 定义了自己的应用协议：

```rust
const ALPN: &[u8] = b"iroh-id-demo/1";
```

ALPN（Application-Layer Protocol Negotiation）用于告诉对端：

> 我要使用哪个应用层协议进行通信。

因此客户端连接时：

```rust
endpoint.connect(server_id, ALPN)
```

服务端也必须声明自己支持同一个 ALPN。

双方使用：

```text
iroh-id-demo/1
```

完成协议协商后，才进入 Demo 自己的应用层通信。

如果客户端和服务端的 ALPN 不匹配，连接会在 QUIC/TLS handshake 阶段失败，例如：

```text
the cryptographic handshake failed:
peer doesn't support any known protocol
```

因此 ALPN 是这个 Demo 中非常重要的一部分。

## 应用层协议

Demo 的应用协议非常简单。

客户端发送：

```text
CLIENT <client-endpoint-id>
```

例如：

```text
CLIENT 4f81yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy
```

服务端回复：

```text
SERVER <server-endpoint-id>
CLIENT <client-endpoint-id>
```

例如：

```text
SERVER 2c7cxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
CLIENT 4f81yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy
```

这只是为了让 Demo 直观地展示双方的身份信息，并不是一个实际应用应该使用的协议格式。

## `remote_id()` 与客户端发送的 ID

服务端收到连接后，可以直接获得 iroh 认证得到的对端身份：

```rust
let remote_id = connection.remote_id();
```

这个 ID 是 iroh 在建立连接时通过密码学身份认证得到的。

因此 Demo 中客户端再次发送：

```text
CLIENT <client-id>
```

实际上是有意重复了一次身份信息。

服务端可以比较：

```text
connection.remote_id()
```

和：

```text
客户端通过应用层发送的 EndpointId
```

理论上二者应该相同。

这也是这个 Demo 一个很有意义的实验点：

```text
             iroh Identity
                   │
                   ▼
             TLS / QUIC
                   │
                   ▼
          connection.remote_id()
                   │
                   │
                   │ compare
                   ▼
        Client application message
                   │
                   ▼
             Client EndpointId
```

如果二者不一致，则说明客户端应用层发送的身份与实际建立连接的密码学身份并不一致。

## 注意

这个 Demo 每次启动都会生成新的 Identity，因此：

```bash
cargo run -- server
```

重新启动后，Server ID 会发生变化。

如果客户端使用旧的 Server ID：

```bash
cargo run -- client <old-server-id>
```

就不能连接到刚刚重新启动的那个 Server。

如果要实现长期存在的服务器身份，应当将服务器的 `SecretKey` 持久化，在下一次启动时重新加载。

## License

本项目仅用于学习和实验 iroh P2P 网络通信。
