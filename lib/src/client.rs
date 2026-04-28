use crate::error::Error;
use crate::error::Result;
#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;
use regex::Regex;
use reqwest::ClientBuilder;
use reqwest::header::CONTENT_TYPE;
use serde_json::Value;
use urlencoding::encode;

#[cfg(feature = "python-bindings")]
use std::fmt::{Display, Formatter};

/// Permit assigned to account, this is account unique.
///
/// For example the regular parking permit for `bewoners`
/// but also `bezoekersparkeren` for visitors.
#[cfg_attr(feature = "python-bindings", pyclass(str, get_all, frozen))]
#[derive(Debug, Clone)]
pub struct Permit {
    /// Id set by parkeerservice for permit
    pub id: u32,
    /// Identifier of product it is, for example `bewoners` or `bezoekers`
    pub product_id: u32,
    /// Nice-name of permit
    pub name: String,
}

impl Permit {
    /// Get the permit from the response json by the parkeerservice.
    pub fn from_json(data: &Value) -> Result<Self> {
        Ok(Self {
            id: data["id"].as_u64().ok_or(Error::InvalidIndex)? as u32,
            product_id: data["permitProductId"]
                .as_u64()
                .ok_or(Error::InvalidIndex)? as u32,
            name: data["permitProduct"]
                .as_str()
                .ok_or(Error::InvalidIndex)?
                .to_string(),
        })
    }
}

#[cfg(feature = "python-bindings")]
impl Display for Permit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}

#[cfg_attr(feature = "python-bindings", pyclass(from_py_object, frozen))]
#[derive(Clone)]
pub struct Client {
    /// Client used for making requests to the endpoint
    request_client: reqwest::Client,
    /// Id of customer
    customer_id: u64,
    /// Endpoint for making requests
    endpoint: String,
    /// Permits assigned to this client
    permits: Vec<Permit>,
}

impl Client {
    /// Initialize the client.
    pub async fn new(
        hostname: Option<String>,
        email: Option<String>,
        password: Option<String>,
    ) -> Result<Self> {
        let request_client = ClientBuilder::new().cookie_store(true).build()?;
        let endpoint = if let Some(url) = hostname {
            url
        } else {
            get_endpoint()?
        };
        let credentials = Credentials::load(email, password)?.to_body();
        let response = request_client.get(&endpoint).send().await?.text().await?;
        let verification_token = get_verification_token(response)?;

        let body = format!("__RequestVerificationToken={verification_token}&{credentials}");
        let response = request_client
            .post(&endpoint)
            .body(body)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(Error::Custom(response.text().await?));
        }
        let customer_data = get_customer_raw_data(&request_client, &endpoint).await?;
        let customer_id = get_customer_id(&customer_data)?;
        let permits = get_permits(&customer_data)?;

        log::info!("Logged in and fetched client succesfully");

        Ok(Self {
            customer_id,
            request_client,
            endpoint,
            permits,
        })
    }

    /// Get a reference to the reqwest client
    pub fn get_reqwest_client(&self) -> &reqwest::Client {
        &self.request_client
    }

    /// Get the customer id of the client
    pub fn get_customer_id(&self) -> u64 {
        self.customer_id
    }

    /// Get the endpoint to the parkeerservice endpoint.
    pub fn get_endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn get_permits(&self) -> &[Permit] {
        &self.permits
    }
}

#[cfg(feature = "python-bindings")]
#[pymethods]
impl Client {
    #[getter]
    /// Get the customer id of the client
    pub fn customer_id(&self) -> u64 {
        self.customer_id
    }

    #[getter]
    /// Get the endpoint to the parkeerservice endpoint.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    #[getter]
    /// Get the permits
    pub fn permits(&self) -> Vec<Permit> {
        self.permits.clone()
    }
}

/// Get the url for the parkeerservice, this is where all calls will be made to.
fn get_endpoint() -> Result<String> {
    match std::env::var("PARKEERSERVICE_ENDPOINT") {
        Ok(endpoint) => Ok(endpoint),
        Err(_) => Err(Error::EnvironmentVariable("PARKEERSERVICE_ENDPOINT url not set. E.g. 'PARKEERSERVICE_ENDPOINT=https://parkstart-LOCATION.parkpermit.eu'.".into()))
    }
}

struct Credentials {
    /// Email of user, e.g. 'example@example.com'
    email: String,
    /// Password of user, e.g. 'verystrong123'
    password: String,
}

impl Credentials {
    /// Load the credentials from the env variables.
    pub fn load(email: Option<String>, password: Option<String>) -> Result<Self> {
        let email = match email {
            Some(email) => email,
            None => std::env::var("PARKEERSERVICE_EMAIL").map_err(|_| {
                Error::EnvironmentVariable(
                    "Email not provided, cannot load credentials. Please set PARKEERSERVICE_EMAIL."
                        .into(),
                )
            })?,
        };
        let password = match password {
            Some(password) => password,
            None => std::env::var("PARKEERSERVICE_PASSWORD").map_err(|_| {
                Error::EnvironmentVariable(
                    "Password not provided, cannot load credentials. Please set PARKEERSERVICE_PASSWORD."
                        .into(),
                )
            })?,
        };
        Ok(Self { email, password })
    }

    /// Get the credentials formatted for the expected body format for logging in.
    pub fn to_body(&self) -> String {
        let email = encode(&self.email);
        let password = encode(&self.password);
        format!("Email={email}&Password={password}")
    }
}

/// Get the verification token from the raw body page, this is fetched from some hidden form.
fn get_verification_token(body: String) -> Result<String> {
    let re = Regex::new(r#"name="__RequestVerificationToken"[^>]*value="([^"]+)""#).unwrap();
    let caps = re.captures(&body).ok_or(Error::VerificationTokenMissing)?;

    let token = caps
        .get(1)
        .map(|m| m.as_str().to_owned())
        .ok_or(Error::VerificationTokenMissing)?;

    Ok(token)
}

async fn get_customer_raw_data(client: &reqwest::Client, endpoint: &str) -> Result<String> {
    Ok(client
        .get(format!("{endpoint}/Customer/PlanSession"))
        .send()
        .await?
        .text()
        .await?)
}

/// Parse the customer id from the customer data page.
fn get_customer_id(raw_body: &str) -> Result<u64> {
    let regex = Regex::new(r#"\\+"customerId\\+":\s*(\d+)"#)?;

    let customer_id = regex
        .captures(raw_body)
        .and_then(|caps| caps.get(1))
        .ok_or(Error::CustomerIdNotAvailable)?
        .as_str()
        .parse::<u64>()?;
    Ok(customer_id)
}

/// Parse the permits from the customer data page.
fn get_permits(raw_body: &str) -> Result<Vec<Permit>> {
    let re = Regex::new(r"planSession\.init\('([^']*)'\)").unwrap();

    let captured = re
        .captures(raw_body)
        .and_then(|c| c.get(1))
        .ok_or(Error::NoPermitsFound)?
        .as_str()
        .replace("\\", "");
    let extracted_json: Value = serde_json::from_str(&captured)?;
    let permit_list = extracted_json
        .get("permitList")
        .and_then(Value::as_array)
        .ok_or(Error::NoPermitsFound)?;

    permit_list.iter().map(Permit::from_json).collect()
}

#[cfg(test)]
mod test {
    use super::*;

    static EXAMPLE_DATA: &str = r#"
    {
        require(['views/customer/planSession/planSession'], function (planSession)
        {
planSession.init('{\"selectedTimelineData\":[],\"permitsForVisualization\":[],\"showSessionTimelines\":true,\"planSessionAdditionalSaldoInfo\":false,\"showPaymentChooser\":false,\"permitList\":[{\"id\":123,\"customerId\":567,\"permitMediaCode\":null,\"cli\":null,\"paidParkingAreaId\":\"BW_B5\",\"status\":\"Active\",\"statusId\":41,\"permitAreaDescription\":null,\"canStopActive\":false,\"timeBalance\":0,\"permitProduct\":\"Bewonersvergunning 1e\",\"ppCode\":\"BW1\",\"ppTranslation\":\"BW1\",\"permitProductId\":99,\"pin\":null,\"sospesPermitPermitId\":\"0123456789\",\"permitAreaId\":\"A1\",\"nprAreaManagerId\":null,\"paidParkingAreaIdDescription\":null,\"permitAreaIdDescription\":null,\"validFromDateUtc\":\"0001-01-01T00:00:00\",\"validUntilDateUtc\":\"2026-09-30T22:00:00\",\"unlimitedTimeBalance\":false,\"isDeleted\":false,\"permitProductAndSospesNumber\":\"Bewonersvergunning 1e / 0123456789\",\"isSynced\":null,\"backOfficeCanAddEditPermit\":false,\"backOfficeCanTerminate\":false,\"numOfSessions\":null,\"allowDynamicNumOfSessions\":false,\"isBusinessPermit\":false,\"displayExpression\":\"Bewonersvergunning 1e - A1 / 0123456789\",\"authSuffix\":\"\",\"totalParkingRights\":0,\"isGaragePermit\":false},{\"id\":112233,\"customerId\":567,\"permitMediaCode\":null,\"cli\":null,\"paidParkingAreaId\":\"A1\",\"status\":\"Active\",\"statusId\":41,\"permitAreaDescription\":null,\"canStopActive\":false,\"timeBalance\":1234,\"permitProduct\":\"Digitale bezoekersregeling\",\"ppCode\":\"BEZREG\",\"ppTranslation\":\"BEZREG\",\"permitProductId\":12,\"pin\":null,\"sospesPermitPermitId\":\"0123456789\",\"permitAreaId\":\"A1\",\"nprAreaManagerId\":null,\"paidParkingAreaIdDescription\":null,\"permitAreaIdDescription\":null,\"validFromDateUtc\":\"0001-01-01T00:00:00\",\"validUntilDateUtc\":\"2075-09-30T22:00:00\",\"unlimitedTimeBalance\":false,\"isDeleted\":false,\"permitProductAndSospesNumber\":\"Digitale bezoekersregeling / 0123456789\",\"isSynced\":null,\"backOfficeCanAddEditPermit\":false,\"backOfficeCanTerminate\":false,\"numOfSessions\":null,\"allowDynamicNumOfSessions\":false,\"isBusinessPermit\":false,\"displayExpression\":\"Digitale bezoekersregeling - A1 / 0123456789\",\"authSuffix\":\"\",\"totalParkingRights\":0,\"isGaragePermit\":false}],\"licensePlates\":[],\"garages\":[],\"addItem\":{\"parkingSessionId\":0,\"permitId\":0,\"timeStart\":\"0001-01-01T00:00:00\",\"timeEnd\":null,\"lp\":null,\"garageCode\":null,\"garageId\":0,\"startedForUser\":567,\"overTimeLimit\":false,\"overTimeLimitText\":null,\"overMoneyLimit\":false,\"overMoneyLimitText\":null,\"endTimeAdjusted\":false,\"endTimeAdjustedText\":null,\"overSessionHourLimit\":false,\"overSessionHourLimitText\":null,\"maxStartEndSessionSpanOverLimit\":false,\"maxStartEndSessionSpanOverLimitText\":null,\"insufficientResources\":false,\"insufficientResourcesText\":null,\"gapNotPassed\":false,\"gapTooShortMessage\":null,\"paidRegimePeriodMessage\":\"\",\"garageSessionMessage\":\"\",\"balanceResetTime\":\"\",\"newEndTimeString\":null,\"psRightId\":null,\"moneyBefore\":null,\"moneyAfter\":null,\"discountCost\":null,\"streetCost\":null,\"timeBefore\":null,\"timeAfter\":null,\"timeCost\":null,\"totalDiscountCost\":0.0,\"totalStreetCost\":0.0,\"totalDurationMinutes\":0,\"discountPercentage\":0.0,\"permitProductId\":0,\"canAddLp\":false,\"unlimitedAndFreePermit\":false,\"isTimeUnlimited\":false,\"isZeroCost\":false,\"planSessionMoneyColumnVisible\":false,\"activePermitsCount\":2,\"saveMode\":false,\"editMode\":false,\"saldo\":13.050000,\"isPermitOwner\":false,\"currentPermitId\":0,\"fromAuthorization\":false,\"customerPays\":true,\"discountCode\":null,\"houseNumber\":null,\"visitorDiscountCodeId\":null,\"permitLabel\":\"Parkeerproduct\"},\"stopSessionModel\":{\"parkingSessionId\":0,\"startDate\":\"0001-01-01T00:00:00\",\"licensePlate\":null,\"permitName\":null,\"permitNumber\":null},\"licensePlate\":{\"id\":0,\"permitId\":0,\"permitProduct\":null,\"lp\":null,\"formattedLP\":null,\"favorite\":false,\"status\":null,\"comment\":null,\"countryCode\":null,\"duplicateCode\":null,\"lpId\":0,\"customerId\":0},\"customerId\":567,\"isCustomer\":true,\"useIvr\":false,\"area\":\"Customer\",\"fillEndTime\":true,\"notifications\":[],\"resources\":{\"cancelPlannedSession\":\"Geplande aanmelding annuleren\",\"cancelPlannedSessionQuestion\":\"Weet u zeker dat u de geplande aanmelding wil annuleren?\",\"comment\":\"Omschrijving\",\"consumption\":\"Verbruik\",\"consumptionAdditonal\":\"Berekening van verbruik o.b.v. bovenstaande periode\",\"decreaseEndTime\":\"Verkort eindtijd\",\"increaseEndTime\":\"Verleng eindtijd\",\"licensePlate\":\"Kenteken\",\"lpCommentPlaceholder\":\"Voer een omschrijving in zoals naam bestuurder\",\"lpSelectCommentPlaceholder\":\"of kies favoriet\",\"lpSelectPlaceholder\":\"Voer kenteken in\",\"money\":\"Geld\",\"paidParkingFrom\":\"betaald parkeren van\",\"permit\":\"Parkeerproduct\",\"permitsForVisualization\":\"Selecteer vergunningen voor visuele preview\",\"savePS\":\"Meld aan\",\"selectEndDate\":\"Selecteer einddatum\",\"selectPermit\":\"Selecteer producttype\",\"selectStartDate\":\"Selecteer startdatum\",\"stopOngoingSession\":\"Lopende aanmelding stoppen\",\"stopOngoingSessionQuestion\":\"Weet u zeker dat u de lopende aanmelding nu wil stoppen?\",\"stopSession\":\"Meld af\",\"timeEnd\":\"Eind\",\"timelineDay\":\"Tijdlijn dag\",\"timelineMonth\":\"Tijdlijn maand\",\"timelineWeek\":\"Tijdlijn week\",\"timeStart\":\"Start\",\"untilOnly\":\"tot\",\"cancel\":\"Annuleren\",\"h\":\"u\",\"invalidMomentEntered\":\"Ongeldig moment opgegeven\",\"noDataCb\":\"Geen data\",\"none\":\"Geen\",\"overMoneyLimit\":\"U heeft onvoldoende geldsaldo om de ingevoerde eind tijd te halen.\",\"overTimeLimit\":\"U heeft onvoldoende tijdsaldo om de ingevoerde eind tijd te halen.\",\"selectCb\":\"Typen...\",\"thisParkingSessionWillCostYou\":\"Kosten van deze aanmelding:\",\"today\":\"Vandaag\",\"unlimitedTimeBalance\":\"Onbeperkt\",\"warning\":\"Waarschuwing\"},\"showTimeCostElements\":false,\"showActiveTimeCostElements\":false,\"showMoneyCostElements\":false,\"showActiveMoneyCostElements\":false,\"hideUnnecessaryUiElements\":false,\"showPaidParkingZones\":false,\"mergeParkingSessions\":false,\"showGarages\":true,\"cancelGarageSessionEnabled\":false,\"useAreaIdDesc\":true,\"userAuthorizationEnabled\":true,\"anonymousVisitorEnabled\":false,\"showChamberOfCommerceNumber\":false,\"languageModel\":{\"selectedLanguage\":\"nl-NL\",\"selectedLanguageName\":\"NL\",\"languageImgClass\":\"nl-NL-img\",\"languages\":[{\"id\":\"en-US\",\"name\":\"EN\",\"image\":\"en-US-img\",\"fullName\":\"English\"},{\"id\":\"nl-NL\",\"name\":\"NL\",\"image\":\"nl-NL-img\",\"fullName\":\"Nederlands\"}]},\"userDisplayName\":\"Example\"}');
        });
    }
    "#;

    #[test]
    fn test_extract_permits() {
        let permits = get_permits(EXAMPLE_DATA).unwrap();

        assert_eq!(permits.len(), 2);
        assert_eq!(permits[0].id, 123);
        assert_eq!(&permits[0].name, "Bewonersvergunning 1e");
        assert_eq!(permits[0].product_id, 99);

        assert_eq!(permits[1].id, 112233);
        assert_eq!(&permits[1].name, "Digitale bezoekersregeling");
        assert_eq!(permits[1].product_id, 12);
    }

    #[test]
    fn test_extract_customer_id() {
        let customer_id = get_customer_id(EXAMPLE_DATA).unwrap();
        assert_eq!(customer_id, 567)
    }
}
