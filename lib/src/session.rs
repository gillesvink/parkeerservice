use crate::{
    Client,
    client::Permit,
    error::{Error, Result},
};
use chrono::{DateTime, NaiveDateTime, Utc};
#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;
use serde_json::{Value, json};

/// License plate object that stores the plate id as well as the description.
#[cfg_attr(feature = "python-bindings", pyclass(str, get_all, frozen))]
#[derive(Debug, Clone)]
pub struct LicensePlate {
    /// Plate id, for example `ABCDEF123`
    pub plate: String,
    /// Description set by user of plate, not always present.
    pub description: Option<String>,
}
#[cfg(feature = "python-bindings")]
impl Display for LicensePlate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?})", self)
    }
}

/// Object that represents a parking session.
#[cfg_attr(feature = "python-bindings", pyclass(str, get_all, frozen))]
#[derive(Debug)]
pub struct Session {
    /// Id of session
    pub id: u32,
    /// Start time of session
    pub start: DateTime<Utc>,
    /// Current end time set of session
    pub end: DateTime<Utc>,
    /// Name of session
    pub name: String,
    /// Flag if session is currently active
    pub active: bool,
    /// Area parking has started
    pub area: String,
    /// Plate assigned to session
    pub license_plate: LicensePlate,
}

impl Session {
    /// Wrap the session from a provided json response into a Session
    pub fn from_json(data: &Value) -> Result<Self> {
        let start = NaiveDateTime::parse_from_str(
            data["timeStartUtc"].as_str().ok_or(Error::InvalidIndex)?,
            "%Y-%m-%dT%H:%M:%S",
        )?
        .and_utc();
        let end = NaiveDateTime::parse_from_str(
            data["timeEndUtc"].as_str().ok_or(Error::InvalidIndex)?,
            "%Y-%m-%dT%H:%M:%S",
        )?
        .and_utc();

        let license_plate_description = data["lpDescription"].as_str().map(|data| data.to_string());
        Ok(Self {
            id: data["id"].as_u64().ok_or(Error::InvalidIndex)? as u32,
            start,
            end,
            name: data["name"]
                .as_str()
                .ok_or(Error::InvalidIndex)?
                .to_string(),
            active: data["isActive"].as_bool().ok_or(Error::InvalidIndex)?,
            area: data["permitAreaDescription"]
                .as_str()
                .ok_or(Error::InvalidIndex)?
                .to_string(),
            license_plate: LicensePlate {
                plate: data["lp"].as_str().ok_or(Error::InvalidIndex)?.to_string(),
                description: license_plate_description,
            },
        })
    }
}

#[cfg(feature = "python-bindings")]
impl Display for Session {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}

/// Get the current active parking sessions.
pub async fn get_parking_sessions(client: &Client) -> Result<Vec<Session>> {
    let (endpoint, customer_id) = (client.get_endpoint(), client.get_customer_id());
    let response = client
        .get_reqwest_client()
        .post(format!(
            "{endpoint}/Customer/SessionsOverview/GetParkingSessions/"
        ))
        .body(format!("cstId={customer_id}"))
        .header(
            reqwest::header::REFERER,
            format!("{endpoint}/Customer/SessionsOverview/Index/"),
        )
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded; charset=UTF-8",
        )
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(Error::Custom(response.text().await?));
    }
    let response_data: Value = response.json().await?;
    let mut data = Vec::new();
    if let Some(entries) = response_data.get("data").and_then(|v| v.as_array()) {
        for entry in entries {
            data.push(Session::from_json(entry)?);
        }
    }

    Ok(data)
}

/// Stop the session for the provided license plate name.
pub async fn stop_session_by_license_plate(client: &Client, license_plate: String) -> Result<()> {
    let sessions = get_parking_sessions(client).await?;
    if let Some(plate) = sessions
        .iter()
        .find(|f| f.license_plate.plate == license_plate)
    {
        stop_session(client, plate.id).await?;
    } else {
        return Err(Error::Custom(
            "License plate was not, could not stop it.".to_string(),
        ));
    }
    Ok(())
}

/// Stop the session for the provided session id.
pub async fn stop_session(client: &Client, session_id: u32) -> Result<()> {
    let endpoint = client.get_endpoint();
    let response = client
        .get_reqwest_client()
        .post(format!(
            "{endpoint}/Customer/SessionsOverview/StopParkingSession"
        ))
        .json(&json!({"parkingSessionId": session_id}))
        .header(
            reqwest::header::REFERER,
            format!("{endpoint}/Customer/SessionsOverview/Index/"),
        )
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(Error::Custom(response.text().await?));
    }

    log::info!("Stopped session successfully.");
    Ok(())
}

/// Start the session for the provided license plate and permit.
pub async fn start_session(
    client: &Client,
    permit: Permit,
    license_plate: LicensePlate,
    end_time: Option<DateTime<Utc>>,
) -> Result<()> {
    let (endpoint, customer_id) = (client.get_endpoint(), client.get_customer_id());

    let end = end_time
        .unwrap_or(
            Utc::now()
                .date_naive()
                .and_hms_opt(23, 59, 59)
                .ok_or(Error::Custom("Creation of time failed".to_string()))?
                .and_utc(),
        )
        .to_rfc3339();
    let response = client
        .get_reqwest_client()
        .post(format!(
            "{endpoint}/Customer/PlanSession/StartParkingSession"
        ))
        .json(&json!({
            "parkingSessionId": 0,
            "permitId": permit.id,
            "lp": license_plate.plate,
            "startedForUser": customer_id,
            "timeStart": Utc::now().to_rfc3339(),
            "timeEnd": end,

        }))
        .header(
            reqwest::header::REFERER,
            format!("{endpoint}/Customer/PlanSession"),
        )
        .header("X-Requested-With", "XMLHttpRequest")
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(Error::Custom(response.text().await?));
    }

    log::info!("Started session successfully.");
    Ok(())
}

/// Start the session for the provided license plate name and permit name.
///
/// This is a convencience function, as it
/// just finds the provided permit by name.
pub async fn start_session_with_license_plate_and_permit_name(
    client: &Client,
    permit: String,
    license_plate: String,
    duration: Option<chrono::Duration>,
) -> Result<()> {
    let permits = client.get_permits();
    let permit = if let Some(permit) = permits.iter().find(|p| p.name == permit) {
        permit
    } else {
        return Err(Error::Custom(
            format!("Permit {permit} could not be found.").to_string(),
        ));
    };
    let license_plate = LicensePlate {
        plate: license_plate,
        description: None,
    };

    let end_time = duration.map(|duration| Utc::now() + duration);

    start_session(client, permit.clone(), license_plate, end_time).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    static TEST_DATA: &str = r#"
    {
        "recordsFiltered": 2,
        "recordsTotal": 2,
        "data": [
            {
                "id": 1,
                "permitId": 1,
                "timeStartUtc": "2025-01-01T00:00:00",
                "timeEndUtc": "2026-01-01T00:00:00",
                "status": null,
                "startMessage": "Automatisch proces",
                "stopMessage": null,
                "costMoney": 0.0,
                "costTime": 525600,
                "parkingRegime": null,
                "hourRate": 0.0,
                "lp": "ABCDEFG",
                "psRightId": "1",
                "name": "Bewonersvergunning 1e",
                "permitNumber": "1",
                "lpName": null,
                "unlimitedTimeBalance": true,
                "isActive": true,
                "isGaragePermit": false,
                "lpDescription": null,
                "permitAreaDescription": "zone A1",
                "zoneCode": "A_A01",
                "authorizationType": -1,
                "email": "example@example.org",
                "fullName": "Example",
                "user": null,
                "userType": null,
                "initials": null,
                "customerPays": true,
                "isPermitOwner": true,
                "visitorName": null,
                "startUserRoleId": 1,
                "discountCode": null,
                "anonymousVisitorDescription": null
            },
            {
                "id": 2,
                "permitId": 1,
                "timeStartUtc": "2025-01-01T00:00:00",
                "timeEndUtc": "2025-01-05T00:00:00",
                "status": null,
                "startMessage": "Gebruiker",
                "stopMessage": null,
                "costMoney": 1.2,
                "costTime": 240,
                "parkingRegime": null,
                "hourRate": null,
                "lp": "XYZ",
                "psRightId": "1",
                "name": "Digitale bezoekersregeling",
                "permitNumber": "1",
                "lpName": null,
                "unlimitedTimeBalance": false,
                "isActive": true,
                "isGaragePermit": false,
                "lpDescription": "My Other Car",
                "permitAreaDescription": "zone A2",
                "zoneCode": "A_A01",
                "authorizationType": -1,
                "email": "example@example.com",
                "fullName": "Example",
                "user": null,
                "userType": null,
                "initials": null,
                "customerPays": true,
                "isPermitOwner": true,
                "visitorName": null,
                "startUserRoleId": 1,
                "discountCode": null,
                "anonymousVisitorDescription": null
            }
        ]
    }
    "#;

    #[test]
    fn test_data_to_sessions() {
        let data: Value = serde_json::from_str(TEST_DATA).unwrap();

        let session1 = Session::from_json(&data["data"][0]).unwrap();
        assert_eq!(
            session1.start,
            DateTime::<Utc>::from_str("2025-01-01T00:00:00Z").unwrap()
        );
        assert_eq!(
            session1.end,
            DateTime::<Utc>::from_str("2026-01-01T00:00:00Z").unwrap()
        );
        assert_eq!(&session1.name, "Bewonersvergunning 1e");
        assert!(session1.active);
        assert_eq!(&session1.area, "zone A1");
        assert_eq!(&session1.license_plate.plate, "ABCDEFG");
        assert_eq!(session1.license_plate.description, None);

        let session2 = Session::from_json(&data["data"][1]).unwrap();
        assert_eq!(
            session2.start,
            DateTime::<Utc>::from_str("2025-01-01T00:00:00Z").unwrap()
        );

        assert_eq!(
            session2.end,
            DateTime::<Utc>::from_str("2025-01-05T00:00:00Z").unwrap()
        );
        assert_eq!(&session2.name, "Digitale bezoekersregeling");
        assert!(session2.active);
        assert_eq!(&session2.area, "zone A2");
        assert_eq!(&session2.license_plate.plate, "XYZ");
        assert_eq!(
            session2.license_plate.description,
            Some("My Other Car".to_string())
        );
    }
}
