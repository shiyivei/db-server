mod frame;
mod multiplex;
mod stream;
mod tls;
use super::*;
use crate::error::KvError;
pub use frame::*;
pub use multiplex::*;
use prost::Message;
pub use stream::*;

mod stream_result;
use stream_result::StreamResult;
pub use tls::*;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

use crate::service::Service;
use tracing::info;

// pub struct ProstServerStream<S> {
//     inner: S,
//     service: Service,
// }
pub struct ProstServerStream<S, Store> {
    inner: ProstStream<S, CommandRequest, CommandResponse>,
    service: Service<Store>,
}

// pub struct ProstClientStream<S> {
//     inner: S,
// }
pub struct ProstClientStream<S> {
    inner: ProstStream<S, CommandResponse, CommandRequest>,
}

impl<S, Store> ProstServerStream<S, Store>
where
    S: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    Store: Storage,
{
    pub fn new(stream: S, service: Service<Store>) -> Self {
        Self {
            inner: ProstStream::new(stream),
            service,
        }
    }

    pub async fn process(mut self) -> Result<(), KvError> {
        // while let Ok(cmd) = self.recv().await {
        //     info!("Got a command {:?}", cmd);

        //     let res = self.service.execute(cmd);
        //     self.send(res).await?;
        // }

        let stream = &mut self.inner;

        while let Some(Ok(cmd)) = stream.next().await {
            info!("Got a new command {:?}", cmd);
            let mut res = self.service.execute(cmd);
            let data = res.next().await.unwrap();
            stream.send(&data).await.unwrap();
        }

        Ok(())
    }

    // async fn send(&mut self, msg: CommandResponse) -> Result<(), KvError> {
    //     let mut buf = BytesMut::new();
    //     msg.encode_frame(&mut buf)?;
    //     let encoded = buf.freeze();
    //     self.inner.write_all(&encoded[..]).await?;

    //     Ok(())
    // }
    // async fn recv(&mut self) -> Result<CommandRequest, KvError> {
    //     let mut buf = BytesMut::new();
    //     let stream = &mut self.inner;

    //     read_frame(stream, &mut buf).await?;

    //     CommandRequest::decode_frame(&mut buf)
    // }
}

impl<S> ProstClientStream<S>
where
    S: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    pub fn new(stream: S) -> Self {
        Self {
            inner: ProstStream::new(stream),
        }
    }

    pub async fn execute_unary(
        &mut self,
        cmd: &CommandRequest,
    ) -> Result<CommandResponse, KvError> {
        // self.send(cmd).await?;
        // Ok(self.recv().await?)

        let stream = &mut self.inner;

        stream.send(cmd).await?;

        match stream.next().await {
            Some(v) => v,
            None => Err(KvError::Internal("Didn't get any response".into())),
        }
    }
    pub async fn execute_streaming(self, cmd: &CommandRequest) -> Result<StreamResult, KvError> {
        // self.send(cmd).await?;
        // Ok(self.recv().await?)

        let mut stream = self.inner;

        stream.send(cmd).await?;
        stream.close().await?;

        StreamResult::new(stream).await
    }

    // async fn send(&mut self, msg: CommandRequest) -> Result<(), KvError> {
    //     let mut buf = BytesMut::new();
    //     msg.encode_frame(&mut buf)?;

    //     let encoded = buf.freeze();

    //     self.inner.write_all(&encoded[..]).await?;

    //     info!("send cmd {:?}", msg);

    //     Ok(())
    // }

    // async fn recv(&mut self) -> Result<CommandResponse, KvError> {
    //     let mut buf = BytesMut::new();
    //     let stream = &mut self.inner;

    //     read_frame(stream, &mut buf).await?;

    //     CommandResponse::decode_frame(&mut buf)
    // }
}

#[cfg(test)]

mod tests {

    use anyhow::Result;

    use bytes::BytesMut;
    use tokio::io::AsyncRead;

    use crate::CommandRequest;

    use super::*;

    struct DummyStream {
        buf: BytesMut,
    }

    impl AsyncRead for DummyStream {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            let len = buf.capacity();

            let data = self.get_mut().buf.split_to(len);

            buf.put_slice(&data);
            std::task::Poll::Ready(Ok(()))
        }
    }

    #[tokio::test]
    async fn read_frame_should_work() {
        let mut buf = BytesMut::new();

        let cmd = CommandRequest::new_hdel("t1", "k1");

        cmd.encode_frame(&mut buf).unwrap();

        let mut stream = DummyStream { buf };

        let mut data = BytesMut::new();

        read_frame(&mut stream, &mut data).await.unwrap();

        let cmd1 = CommandRequest::decode_frame(&mut data).unwrap();

        assert_eq!(cmd1, cmd)
    }

    #[tokio::test]

    async fn client_server_basic_communication_should_work() -> anyhow::Result<()> {
        let addr = start_server().await?;
        let stream = TcpStream::connect(addr).await?;

        let mut client = ProstClientStream::new(stream);

        let cmd = CommandRequest::new_hset("t1", "k1", "v1".into());
        let res = client.execute_unary(&cmd).await.unwrap();

        assert_res_ok(&res, &[Value::default()], &[]);

        let cmd = CommandRequest::new_hget("t1", "k1");
        let res = client.execute_unary(&cmd).await?;

        assert_res_ok(&res, &["v1".into()], &[]);

        Ok(())
    }

    #[tokio::test]
    async fn client_server_compression_should_work() -> anyhow::Result<()> {
        let addr = start_server().await?;
        let stream = TcpStream::connect(addr).await?;
        let mut client = ProstClientStream::new(stream);
        let v: Value = Bytes::from(vec![0u8; 16384]).into();
        let cmd = CommandRequest::new_hset("t2", "k2", v.clone().into());
        let res = client.execute_unary(&cmd).await?;
        assert_res_ok(&res, &[Value::default()], &[]);
        let cmd = CommandRequest::new_hget("t2", "k2");
        let res = client.execute_unary(&cmd).await?;
        assert_res_ok(&res, &[v.into()], &[]);
        Ok(())
    }

    async fn start_server() -> Result<SocketAddr> {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();

                let service: Service = ServiceInner::new(MemTable::new()).into();
                let server = ProstServerStream::new(stream, service);
                tokio::spawn(server.process());
            }
        });

        Ok(addr)
    }
}
