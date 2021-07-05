use logicbatch::tag_server_capnp::tagger;
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};

use futures::AsyncReadExt;

use futures::FutureExt;

pub async fn main(sa: std::net::SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    tokio::task::LocalSet::new().run_until(async move {
        let stream = tokio::net::TcpStream::connect(&sa).await?;
        stream.set_nodelay(true)?;
        let (reader, writer) = tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
        let rpc_network = Box::new(twoparty::VatNetwork::new(
            reader,
            writer,
            rpc_twoparty_capnp::Side::Client,
            Default::default(),
        ));
        let mut rpc_system = RpcSystem::new(rpc_network, None);
        let t: tagger::Client =
            rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

        tokio::task::spawn_local(Box::pin(rpc_system.map(|_| ())));

        let mut job = t.submitjob_request();
        let mut jb = job.get().init_job();
        let mut pb = jb.reborrow().init_patterns(3);
        pb.set(0, 0b0000_0000_0000_0001);
        pb.set(1, 0b0000_0000_0000_0010);
        pb.set(2, 0b0000_0000_0000_0011);
        jb.reborrow().set_duration(200_000_000);

        println!("sending job");

        let rdr = job.send().promise.await.unwrap();
        let jid = match rdr.get().unwrap().get_sub().unwrap().which() {
            Ok(logicbatch::tag_server_capnp::job_submission::Which::Badsub(_)) => None,
            Ok(logicbatch::tag_server_capnp::job_submission::Which::Jobid(i)) => Some(i),
            Err(_) => None,
        };
        tokio::time::sleep(
            std::time::Duration::from_millis(1000)
        ).await;

        println!("check in on job");

        if let Some(id) = jid {
            let r = loop {
                let mut q = t.queryjobdone_request();
                q.get().set_jobid(id);
                let qrdr = q.send().promise.await.unwrap();
                match qrdr.get().unwrap().get_ret().unwrap() {
                    logicbatch::tag_server_capnp::JobStatus::Waiting => {
                        tokio::time::sleep(
                            std::time::Duration::from_millis(100)
                        ).await;
                    },
                    logicbatch::tag_server_capnp::JobStatus::Ready => {
                        break Some(())
                    },
                    _ => break None,

                }
            };
            if let Some(()) = r {
                let mut ans = t.getresults_request();
                ans.get().set_jobid(id);
                let ardr = ans.send().promise.await.unwrap();
                match ardr.get().unwrap().get_payload().unwrap().which() {
                    Ok(logicbatch::tag_server_capnp::job_payload::Badquery(_)) => println!("badquery"),
                    Ok(logicbatch::tag_server_capnp::job_payload::Payload(p)) => {
                        let mut ev = Vec::<u64>::new();
                        let mut pt = Vec::<u16>::new();
                        let prdr = p.unwrap();
                        let er = prdr.reborrow().get_patterns().unwrap();
                        for x in er {
                            pt.push(x);
                        }
                        let pr = prdr.reborrow().get_events().unwrap();
                        for y in pr {
                            ev.push(y);
                        }
                        for (&i, &j) in pt.iter().zip(ev.iter()) {
                            println!("Pattern: {0} Counts: {1}", i, j);
                        }
                    },
                    Err(_) => println!("ans not in schema"),
                }
            } else {
                println!("Didn't work");
            }
        }

        Ok(())
    }).await
}