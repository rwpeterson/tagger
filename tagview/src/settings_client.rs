use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::{AsyncReadExt, FutureExt};
use tagger_capnp::tag_server_capnp::{publisher, service_pub};
use tokio::runtime::Builder;
use tokio::sync::mpsc;

struct SettingsClient {
    receiver: mpsc::UnboundedReceiver<SettingsMessage>,
}

pub enum SettingsMessage {
    Get {
        respond_to: flume::Sender<RawChannelState>,
    },
    Set {
        setting: RawChannelSetting,
        respond_to: flume::Sender<()>,
    },
    Shutdown(),
}

pub struct RawChannelState {
    pub invm: u16,
    pub dels: Vec<u32>,
    pub thrs: Vec<f64>,
}

pub enum RawChannelSetting {
    Inversion((u8, bool)),
    Delay((u8, u32)),
    Threshold((u8, f64)),
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct RawSingleChannelState {
    pub ch: u8,
    pub inv: bool,
    pub del: u32,
    pub thr: f64,
}

impl SettingsClient {
    fn new(receiver: mpsc::UnboundedReceiver<SettingsMessage>) -> Self {
        SettingsClient { receiver }
    }
}

#[derive(Clone)]
pub struct SettingsClientHandle {
    pub sender: mpsc::UnboundedSender<SettingsMessage>,
}

impl SettingsClientHandle {
    pub fn new(addr: std::net::SocketAddr) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let mut rpc_client = SettingsClient::new(receiver);
        let rt = Builder::new_current_thread().enable_all().build().unwrap();

        std::thread::spawn(move || {
            rt.block_on(async move {
                rpc_client.main(addr).await.unwrap();
            });
        });

        SettingsClientHandle { sender }
    }
}

impl SettingsClient {
    async fn main(
        &mut self,
        addr: std::net::SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error>> {
        tokio::task::LocalSet::new()
            .run_until(async move {
                // Manages the network connection and abstracts it into a Cap'n Proto RPC system
                let stream = tokio::net::TcpStream::connect(&addr).await?;
                stream.set_nodelay(true)?;
                let (reader, writer) =
                    tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
                let rpc_network = Box::new(twoparty::VatNetwork::new(
                    reader,
                    writer,
                    rpc_twoparty_capnp::Side::Client,
                    Default::default(),
                ));
                let mut rpc_system = RpcSystem::new(rpc_network, None);

                // We don't use service_pub, but publisher is a template so we need to use something
                let publisher: publisher::Client<service_pub::Owned> =
                    rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

                let _ = tokio::task::spawn_local(Box::pin(rpc_system.map(|_| ())));
                
                // Now manage channel get/set requests until program is terminated
                loop {
                    match self.receiver.recv().await {
                        Some(msg) => {
                            match msg {
                                SettingsMessage::Get { respond_to } => {
                                    let req = publisher.get_inputs_request();
                                    let reply = req.send().promise.await?;
                                    let rdr = reply.get().unwrap().get_s().unwrap();
                                    respond_to.send(
                                        RawChannelState {
                                            invm: rdr.get_inversionmask(),
                                            dels: rdr.get_delays().unwrap().iter().collect(),
                                            thrs: rdr.get_thresholds().unwrap().iter().collect(),
                                        }
                                    )?;
                                },
                                SettingsMessage::Set { setting, respond_to } => {
                                    let mut req = publisher.set_input_request();
                                    match setting {
                                        RawChannelSetting::Inversion((ch, inv)) => {
                                            let mut rbdr = req.get().init_s().init_inversion();
                                            rbdr.set_ch(ch);
                                            rbdr.set_inv(inv);
                                        },
                                        RawChannelSetting::Delay((ch, del)) => {
                                            let mut rbdr = req.get().init_s().init_delay();
                                            rbdr.set_ch(ch);
                                            rbdr.set_del(del);
                                        },
                                        RawChannelSetting::Threshold((ch, th)) => {
                                            let mut rbdr = req.get().init_s().init_threshold();
                                            rbdr.set_ch(ch);
                                            rbdr.set_th(th);
                                        },
                                    }
                                    let _ = req.send().promise.await?;
                                    respond_to.send(())?;
                                },
                                SettingsMessage::Shutdown() => break,
                            }
                        },
                        None => break,
                    }
                }
                Ok(())
            }
        ).await
    }
}
