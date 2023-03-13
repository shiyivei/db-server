use anyhow::Result;
use async_prost::AsyncProstStream;
use db_server::{CommandRequest, CommandResponse, MemTable, Service, ServiceInner};
use futures::prelude::*;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 创建数据结构
    let service: Service = ServiceInner::new(MemTable::default()).into();

    // 监听请求
    let addr = "127.0.0.1:9527";
    let listener = TcpListener::bind(addr).await?;
    info!("Start listening on {:?}", addr);

    loop {
        // 拿到tcp数据流
        let (stream, addr) = listener.accept().await?;
        info!("Client {:?} connected", addr);

        let svc = service.clone();

        tokio::spawn(async move {
            let mut async_stream =
                AsyncProstStream::<_, CommandRequest, CommandResponse, _>::from(stream).for_async();

            while let Some(Ok(cmd)) = async_stream.next().await {
                info!("Got a new command: {:?}", cmd);
                let mut res = svc.execute(cmd);

                while let Some(data) = res.next().await {
                    async_stream.send((*data).clone()).await.unwrap();
                }
            }
            info!("Client {:?} disconnected", addr)
        });
    }
}
