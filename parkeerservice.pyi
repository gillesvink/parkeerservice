from datetime import datetime

class LicensePlate:
    """
    License plate object that stores the plate id as well
    as the description.
    """

    plate: str
    """Plate id, for example `ABCDEF123`"""
    description: None | str
    """Description set by user of plate, not always present."""

class Session:
    """Object that represents a parking session."""

    id: int
    """Id of session"""
    start: datetime
    """Start time of session"""
    end: datetime
    """Current end time set of session"""
    name: str
    """Name of session"""
    active: bool
    """Flag if session is currently active"""
    area: str
    """Area parking has started"""
    license_plate: LicensePlate
    """Plate assigned to session"""

class Permit:
    """
    Permit assigned to account, this is account unique.

    For example the regular parking permit for `bewoners`
    but also `bezoekersparkeren` for visitors.
    """

    id: int
    """Id set by parkeerservice for permit"""
    product_id: int
    """Identifier of product it is, for example "bewoners" or "bezoekers"""
    name: str
    """Nice-name of permit"""

class Client:
    """Object that is used to make requests to the service."""

    @property
    def customer_id(self) -> int:
        """Customer id for this user."""

    @property
    def permits(self) -> list[Permit]:
        """Permits assigned to this user."""

    @property
    def endpoint(self) -> int:
        """Endpoint used for making requests."""

async def get_client(
    hostname: str | None = None, email: str | None = None, password: str | None = None
) -> Client:
    """Get the client to make requests with to the parkeerservice.

    The following environment variables can be set beforehand:

    * `PARKEERSERVICE_EMAIL` the email used for logging in
    * `PARKEERSERVICE_PASSWORD` the password used for logging in
    * `PARKEERSERVICE_ENDPOINT` the endpoint url for the parkeerservice, e.g. https://parkstart-LOCATION.parkpermit.eu

    Raises:
        RuntimeError: if something goes wrong with requesting or the environment variables are not set.
    """

async def get_sessions(client: Client) -> list[Session]:
    """Get the current active parking sessions.

    Args:
        client: client to make requests with to the service

    Raises:
        RuntimeError: if something goes wrong internally.
    """

async def stop(client: Client, license_plate: str):
    """Stop the session for the provided license plate name.

    Args:
        client: client to make requests with to the service

    Raises:
        RuntimeError: if something goes wrong internally.
    """

async def start(
    client: Client,
    license_plate: str,
    permit: str,
    duration: int | None = None,
):
    """Start the session for the provided license plate name and permit name.

    Args:
        client: client to make requests with to the service
        license_plate: plate to start session for
        permit: name of permit, must match exactly with permit name
        duration (optional): time in seconds for parking session.

    Raises:
        RuntimeError: if something goes wrong internally.
    """
