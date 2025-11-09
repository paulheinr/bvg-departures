use crate::api::BvgClient;
use crate::api::departures::DeparturesParams;

mod api;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = BvgClient::default();

    let params = DeparturesParams {
        duration: Some(10),
        lines_of_stops: Some(false),
        remarks: Some(true),
        language: Some("en".into()),
        ..Default::default()
    };

    let resp = client.get_departures("900055151", &params).await?;
    for d in resp.departures.iter().take(8) {
        let line = d.line.as_ref().and_then(|l| l.name.as_ref()).map(String::as_str).unwrap_or("?");
        let dir  = d.direction.as_deref().unwrap_or("");
        let mins = d.when.map(|w| (w - chrono::Utc::now()).num_seconds() / 60);
        println!("{:<6} {:<35} {:>3} min",
                 line,
                 dir,
                 mins.unwrap_or_default().max(0)
        );
    }
    Ok(())
}