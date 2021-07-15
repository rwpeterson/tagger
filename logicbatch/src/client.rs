use tagger_capnp::tag_server_capnp::{tagger, job_payload, job_submission, JobStatus};
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};

use futures::AsyncReadExt;

use futures::FutureExt;

use anyhow::Result;

pub async fn main(addr: std::net::SocketAddr) -> Result<()> {
    let t_secs = 20;
    let the_id = tokio::task::LocalSet::new().run_until(async move {
        let stream = tokio::net::TcpStream::connect(&addr).await?;
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
        jb.reborrow().set_duration(200_000_000 * t_secs);

        println!("sending job");

        let rdr = job.send().promise.await.unwrap();
        let jid = match rdr.get().unwrap().get_sub().unwrap().which() {
            Ok(job_submission::Which::Badsub(_)) => None,
            Ok(job_submission::Which::Jobid(i)) => Some(i),
            Err(_) => None,
        };
        tokio::time::sleep(
            std::time::Duration::from_secs(t_secs + 1)
        ).await;
        Ok(jid.unwrap()) as Result<u64>
    }).await.unwrap();

    let chk = tokio::task::LocalSet::new().run_until(async move {
        let stream = tokio::net::TcpStream::connect("127.0.0.1:6969").await?;
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

        println!("check in on job");

        let mut q = t.queryjobdone_request();
        q.get().set_jobid(the_id);
        let qrdr = q.send().promise.await?;
        let check = match qrdr.get()?.get_ret()? {
            JobStatus::Ready => Some(()),
            _ => None,
        };
        Ok(check) as Result<Option<()>>
    }).await?;

    assert_eq!(chk, Some(()));
    println!("job is ready for us");

    tokio::task::LocalSet::new().run_until(async move {
        let stream = tokio::net::TcpStream::connect("127.0.0.1:6969").await?;
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

        println!("get job results");

        let mut ans = t.getresults_request();
        ans.get().set_jobid(the_id);
        let ardr = ans.send().promise.await.unwrap();
        match ardr.get().unwrap().get_payload().unwrap().which() {
            Ok(job_payload::Badquery(_)) => println!("badquery"),
            Ok(job_payload::Payload(p)) => {
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
                let dur = prdr.reborrow().get_duration() as f64 / 200_000_000f64;
                for (&i, &j) in pt.iter().zip(ev.iter()) {
                    println!("Pattern: {0} Rate: {1}", i, j as f64 / dur);
                }
            },
            Err(_) => println!("ans not in schema"),
        }

        Ok(()) as Result<()>
    }).await
}