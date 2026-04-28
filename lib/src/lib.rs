mod error;
mod session;
pub use error::{Error, Result};

mod client;
pub use client::Client;
pub use session::{
    get_parking_sessions, start_session, start_session_with_license_plate_and_permit_name,
    stop_session, stop_session_by_license_plate,
};

#[cfg(feature = "python-bindings")]
#[pyo3::pymodule]
mod parkeerservice {
    use chrono::Duration;
    use pyo3::prelude::*;

    use crate::{
        get_parking_sessions, start_session_with_license_plate_and_permit_name,
        stop_session_by_license_plate,
    };

    #[pymodule_export]
    use crate::{client::Client, client::Permit, session::LicensePlate, session::Session};

    #[pyfunction]
    #[pyo3(name = "get_client")]
    fn get_client_py(
        py: Python,
        hostname: Option<String>,
        email: Option<String>,
        password: Option<String>,
    ) -> PyResult<Bound<PyAny>> {
        let _ = pyo3_log::try_init();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            Ok(Client::new(hostname, email, password).await?)
        })
    }

    #[pyfunction]
    #[pyo3(name = "start")]
    fn start_py(
        py: Python,
        client: Client,
        license_plate: String,
        permit: String,
        duration: Option<i64>,
    ) -> PyResult<Bound<PyAny>> {
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let duration_delta = Duration::seconds(duration.unwrap());
            start_session_with_license_plate_and_permit_name(
                &client,
                permit,
                license_plate,
                Some(duration_delta),
            )
            .await?;
            Ok(())
        })
    }

    #[pyfunction]
    #[pyo3(name = "stop")]
    fn stop_py(py: Python, client: Client, license_plate: String) -> PyResult<Bound<PyAny>> {
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            stop_session_by_license_plate(&client, license_plate).await?;
            Ok(())
        })
    }

    #[pyfunction]
    #[pyo3(name = "get_sessions")]
    fn get_sessions_py(py: Python, client: Client) -> PyResult<Bound<PyAny>> {
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            Ok(get_parking_sessions(&client).await?)
        })
    }
}
