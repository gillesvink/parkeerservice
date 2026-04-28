"""Automation switch to link the parkeerservice to a HA switch"""

from __future__ import annotations

import logging
from typing import Final
import contextlib
from attr import dataclass
import parkeerservice
import voluptuous as vol
from datetime import datetime
import homeassistant.helpers.config_validation as cv
from homeassistant.components.switch import SwitchEntity, PLATFORM_SCHEMA
from homeassistant.const import CONF_HOST, CONF_PASSWORD, CONF_EMAIL, CONF_DESCRIPTION
from homeassistant.core import HomeAssistant
from homeassistant.helpers.entity_platform import AddEntitiesCallback
from homeassistant.helpers.typing import ConfigType, DiscoveryInfoType

_LOGGER = logging.getLogger(__name__)

CONF_CARS: Final = "cars"
CONF_LICENSE_PLATE: Final = "license_plate"
CONF_PERMIT: Final = "permit"

PLATFORM_SCHEMA = PLATFORM_SCHEMA.extend(
    {
        vol.Required(CONF_HOST): cv.string,
        vol.Required(CONF_EMAIL): cv.string,
        vol.Required(CONF_PASSWORD): cv.string,
        vol.Required(CONF_CARS): vol.All(
            cv.ensure_list,
            [
                vol.Schema(
                    {
                        vol.Required(CONF_DESCRIPTION): cv.string,
                        vol.Required(CONF_LICENSE_PLATE): cv.string,
                        vol.Required(CONF_PERMIT): cv.string,
                    }
                )
            ],
        ),
    }
)


@dataclass
class Credentials:
    host: str
    email: str
    password: str


async def async_setup_platform(
    hass: HomeAssistant,
    config: ConfigType,
    add_entities: AddEntitiesCallback,
    discovery_info: DiscoveryInfoType | None = None,
) -> None:
    credentials = Credentials(
        config[CONF_HOST], config[CONF_EMAIL], config[CONF_PASSWORD]
    )
    try:
        client = await parkeerservice.get_client(
            credentials.host, credentials.email, credentials.password
        )
    except RuntimeError as error:
        _LOGGER.error("Could not get client '%s'.", error)
        return

    sessions = [ParkingSession(car, credentials, client) for car in config[CONF_CARS]]
    for session in sessions:
        await session.async_update()
    add_entities(sessions)


class ParkingSession(SwitchEntity):
    """Representation of a a car that could park."""

    def __init__(
        self, car: ConfigType, credentials: Credentials, client: parkeerservice.Client
    ) -> None:
        """Initialize an AwesomeLight."""
        self._description = car[CONF_DESCRIPTION]
        self._license_plate = car[CONF_LICENSE_PLATE]
        self._permit = car[CONF_PERMIT]
        self._credentials = credentials
        self._active = False
        self._last_update = datetime.now()
        self._client = client

    @property
    def name(self) -> str:
        """Return the display name of this light."""
        return self._description

    @property
    def is_on(self) -> bool | None:
        """Return true if light is on."""
        return self._active

    async def async_turn_on(self, **kwargs: Any) -> None:
        """Start the park action"""
        self._active = True

    async def async_turn_off(self, **kwargs: Any) -> None:
        """Stop the park action"""
        self._active = False

    async def _get_client(self) -> parkeerservice.Client:
        if (
            datetime.now() - self._last_update
        ).seconds > 60 * 3:  # as the keys are pretty short-lived, refresh after 3 mins
            self._client = await parkeerservice.get_client(
                self._credentials.host,
                self._credentials.email,
                self._credentials.password,
            )
            self._last_update = datetime.now()
        return self._client

    async def async_update(self) -> None:
        """Fetch status of session and set active state."""
        try:
            parking_sessions = await parkeerservice.get_sessions(
                await self._get_client()
            )
        except RuntimeError as error:
            _LOGGER.error(error)
            self._active = False
        active = False
        with contextlib.suppress(StopIteration):
            session = next(
                session
                for session in parking_sessions
                if self._license_plate == session.license_plate.plate
            )
            active = session.active
        self._active = active
