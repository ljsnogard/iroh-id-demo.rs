use anyhow::{bail, Context, Result};
use iroh::{Endpoint, EndpointId, endpoint::presets};
use std::env;

const ALPN: &[u8] = b"iroh-id-demo/1";

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = env::args().skip(1);

    let mode = args
        .next()
        .context("missing mode: use `server` or `client`")?;

    match mode.as_str() {
        "server" => run_server().await,
        "client" => {
            let server_id = args
                .next()
                .context("missing server ID")?
                .parse::<EndpointId>()
                .context("invalid server endpoint ID")?;

            run_client(server_id).await
        }
        _ => {
            bail!(
                "unknown mode `{mode}`\n\
                 usage:\n\
                 \t{} server\n\
                 \t{} client <server-id>",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_NAME"),
            );
        }
    }
}

/// 启动服务端。
///
/// 服务端的身份由 iroh 在创建 Endpoint 时随机生成。
async fn run_server() -> Result<()> {
    // N0 preset 会配置 iroh 所需的默认网络能力，
    // 包括 address lookup 等，使客户端可以仅凭 EndpointId
    // 尝试找到服务端。
    let endpoint = Endpoint::builder(presets::N0)
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await?;

    let server_id = endpoint.id();

    println!("Server started.");
    println!("Server ID:");
    println!("{server_id}");
    println!();
    println!("Waiting for client...");

    // 等待一个客户端连接。
    //
    // accept() 首先等待一个 incoming connection，
    // 然后再 await 它完成 QUIC/iroh 握手。
    let incoming = endpoint
        .accept()
        .await
        .context("failed to accept incoming connection")?;

    let connection = incoming
        .await
        .context("failed to establish connection")?;

    println!("Client connected.");
    println!("Remote endpoint: {}", connection.remote_id());

    // 建立一个双向 QUIC stream。
    let (mut send, mut recv) = connection
        .accept_bi()
        .await
        .context("failed to accept bidirectional stream")?;

    // 读取客户端发送的身份。
    let data = recv
        .read_to_end(1024)
        .await
        .context("failed to read client identity")?;

    let message = String::from_utf8(data)
        .context("client message is not valid UTF-8")?;

    let client_id = message
        .strip_prefix("CLIENT ")
        .context("invalid client message")?
        .trim()
        .parse::<EndpointId>()
        .context("client sent an invalid endpoint ID")?;

    println!("Client reported ID:");
    println!("{client_id}");

    // 服务端回复自己的身份，以及刚刚收到的客户端身份。
    let response = format!(
        "SERVER {server_id}\nCLIENT {client_id}\n"
    );

    send.write_all(response.as_bytes())
        .await
        .context("failed to send response")?;

    send.finish()
        .context("failed to finish send stream")?;

    println!();
    println!("Response sent.");
    println!("Server ID : {server_id}");
    println!("Client ID : {client_id}");

    // 等待连接关闭。
    connection.closed().await;

    endpoint.close().await;

    Ok(())
}

/// 启动客户端。
///
/// 客户端启动时同样会随机生成自己的 iroh identity，
/// 但它还需要知道服务端的 EndpointId，才能主动发起连接。
async fn run_client(server_id: EndpointId) -> Result<()> {
    let endpoint = Endpoint::bind(presets::N0).await?;

    let client_id = endpoint.id();

    println!("Client started.");
    println!("Client ID:");
    println!("{client_id}");
    println!();
    println!("Connecting to server:");
    println!("{server_id}");

    // 使用服务端 EndpointId 建立 iroh 连接。
    //
    // ALPN 是我们这个 demo 自己定义的应用层协议标识。
    let connection = endpoint
        .connect(server_id, ALPN)
        .await
        .context("failed to connect to server")?;

    println!("Connected.");

    // 创建双向 stream。
    let (mut send, mut recv) = connection
        .open_bi()
        .await
        .context("failed to open bidirectional stream")?;

    // 把自己的 EndpointId 发送给服务端。
    let message = format!("CLIENT {client_id}");

    send.write_all(message.as_bytes())
        .await
        .context("failed to send client identity")?;

    send.finish()
        .context("failed to finish send stream")?;

    // 接收服务端响应。
    let response = recv
        .read_to_end(1024)
        .await
        .context("failed to read server response")?;

    let response = String::from_utf8(response)
        .context("server response is not valid UTF-8")?;

    println!();
    println!("Server response:");
    println!("{response}");

    connection.close(0u32.into(), b"done");
    endpoint.close().await;

    Ok(())
}
