use anyhow::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (pack_num_min, pack_num_max, pack_path) = get_packer_args();
    println!(
        r"
        _____ __    _           _ __
       / ___// /_  (_)___ ___  (_) /______
       \__ \/ __ \/ / __ `__ \/ / //_/ __ \
      ___/ / / / / / / / / / / / ,< / /_/ /
     /____/_/ /_/_/_/ /_/ /_/_/_/|_|\____/

    "
    );
    let pack_path_arc = std::sync::Arc::new(pack_path);

    std::fs::create_dir_all(pack_path_arc.clone().as_ref())
        .expect("failed to create directory at path");
    let http_client = reqwest::Client::new();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<shimiko::task::PackerTask>(10);
    let (fail_tx, mut fail_rx) = tokio::sync::mpsc::channel::<shimiko::task::PackerTaskFail>(3000);
    let fail_tx_manager = fail_tx.clone();
    let packer = tokio::spawn(async move {
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(3));
        let mut join_set = tokio::task::JoinSet::new();
        while let Some(packer_task) = rx.recv().await {
            let permit = semaphore
                .clone()
                .acquire_owned()
                .await
                .expect("failed to acquire semaphore permit");
            let path_pack = pack_path_arc.clone().to_path_buf();
            let fail_tx_task = fail_tx_manager.clone();
            join_set.spawn_blocking(move || {
                let _permit = permit;
                packer_task.progress_bar().set_message("Extracting...");
                match packer_task.extract(&path_pack) {
                    Ok(()) => {
                        packer_task
                            .progress_bar()
                            .finish_with_message("Extracting...Finished");
                    }
                    Err(e) => {
                        packer_task
                            .progress_bar()
                            .finish_with_message("Extracting...Failed");
                        fail_tx_task
                            .blocking_send(shimiko::task::PackerTaskFail::new(
                                *packer_task.pack_num(),
                                e,
                            ))
                            .expect("failed to send error through channel");
                    }
                };
            });
        }

        while let Some(result) = join_set.join_next().await {
            result.unwrap();
        }
    });

    let mpb = indicatif::MultiProgress::new();
    let pb_style =
        indicatif::ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
            .expect("failed to parse progress bar template");

    for i in pack_num_min..=pack_num_max {
        let pb = mpb.add(indicatif::ProgressBar::new(2));
        pb.set_style(pb_style.clone());
        pb.set_prefix(format!("[Pack #{i}]:"));
        pb.enable_steady_tick(std::time::Duration::from_millis(100));

        let packer_task = match shimiko::task::PackerTask::new(i, &http_client, pb)
            .await
            .with_context(|| "failed to create packer task")
        {
            Ok(packer_task) => packer_task,
            Err(e) => {
                fail_tx
                    .send(shimiko::task::PackerTaskFail::new(i, e))
                    .await
                    .expect("failed to send error through channel");
                continue;
            }
        };

        if let Err(e) = tx
            .send(packer_task)
            .await
            .with_context(|| "failed to send task through channel")
        {
            println!("{e:#}");
            continue;
        }
    }

    drop(tx);
    packer
        .await
        .with_context(|| "something went wrong with async workers")
        .unwrap();
    drop(fail_tx);
    while let Some(fail_task) = fail_rx.recv().await {
        println!(
            "Pack {} failed due to following: {:#}",
            fail_task.pack_num(),
            fail_task.error()
        );
    }
    Ok(())
}

fn get_packer_args() -> (u16, u16, std::path::PathBuf) {
    let cmd = clap::Command::new("shimiko")
        .version("1.0")
        .about("beatmap pack downloader")
        .arg(
            clap::arg!([pack_num_min] "beginning range of pack")
                .value_parser(clap::value_parser!(u16))
                .required(true),
        )
        .arg(
            clap::arg!([pack_num_max] "end range of pack")
                .value_parser(clap::value_parser!(u16))
                .required(true),
        )
        .arg(
            clap::arg!([pack_path] "extraction path")
                .value_parser(clap::value_parser!(std::path::PathBuf))
                .required(true),
        )
        .get_matches();

    let mut pack_num_min = cmd
        .get_one::<u16>("pack_num_min")
        .expect("minimum range of packs to download is missing")
        .to_owned();
    let mut pack_num_max = cmd
        .get_one::<u16>("pack_num_max")
        .expect("maximum range of packs to download is missing")
        .to_owned();
    let pack_path = cmd
        .get_one::<std::path::PathBuf>("pack_path")
        .expect("output path is missing")
        .to_owned();

    assert!(pack_num_min > 0, "pack range cannot be 0");
    assert!(pack_num_max > 0, "pack range cannot be 0");

    if pack_num_min > pack_num_max {
        std::mem::swap(&mut pack_num_min, &mut pack_num_max);
    }

    assert!(
        pack_num_max < 3000,
        "pack maximum range is too high: {pack_num_max}"
    );

    (pack_num_min, pack_num_max, pack_path)
}
