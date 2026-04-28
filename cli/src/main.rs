use clap::{ArgAction, Parser};
use parkeerservice::{Client, Result};
use parkeerservice::{
    get_parking_sessions, start_session_with_license_plate_and_permit_name,
    stop_session_by_license_plate,
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, arg_required_else_help = true)]
struct Args {
    /// The license plate name of the car to add, e.g. "ERRP10".
    #[clap(long, value_name = "LICENSE_PLATE")]
    start: Option<String>,

    /// The license plate name of the car to add, e.g. "ERRP10".
    #[clap(long, value_name = "LICENSE_PLATE")]
    stop: Option<String>,

    /// e.g. "Digitale bezoekersregeling"
    #[clap(long, value_name = "PERMIT_NAME")]
    permit: Option<String>,

    /// Optionally assign the duration for the parking session in seconds.
    #[clap(long, value_name = "SECONDS")]
    duration: Option<u64>,

    /// Get all current active parking sessions.
    #[clap(long, action=ArgAction::SetTrue)]
    get_sessions: bool,

    /// Get the permits assigned to this account
    #[clap(long, action=ArgAction::SetTrue)]
    get_permits: bool,
}

async fn program() -> Result<()> {
    let args = Args::parse();

    let client = Client::new(None, None, None).await?;
    if args.get_sessions {
        let sessions = get_parking_sessions(&client).await?;
        for session in sessions {
            println!("{:#?}", session);
        }
        return Ok(());
    }

    if args.get_permits {
        let permits = client.get_permits();
        for permit in permits {
            println!("{:#?}", permit);
        }
        return Ok(());
    }
    if let Some(plate) = args.stop {
        stop_session_by_license_plate(&client, plate).await?;
        return Ok(());
    }

    if let Some(plate) = args.start
        && let Some(permit) = args.permit {
            let duration = args.duration.map(|duration| chrono::Duration::seconds(duration as i64));

            start_session_with_license_plate_and_permit_name(&client, permit, plate, duration)
                .await?;
            return Ok(());
        }
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt()
        .compact()
        .with_line_number(false)
        .with_file(false)
        .init();
    match program().await {
        Ok(_) => (),
        Err(error) => log::error!("{}", error),
    }
}
