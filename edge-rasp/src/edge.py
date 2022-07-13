import asyncio
import logging

import toml
from pydantic import ValidationError

from model import CommandState
from http_server import run_http_server
from mqtt import Config, run_mqtt
from db import run_db
from coap import run_coap

logging.basicConfig(level=logging.DEBUG)


async def main():
    command_state = CommandState.default()
    command_state_lock = asyncio.Lock()
    received_data_points_mqtt = asyncio.Queue()
    received_data_points_db = asyncio.Queue()

    try:
        config_dict = toml.load("config.toml")
        config = Config.parse_obj(config_dict)

        await asyncio.gather(run_db(config, received_data_points_db),
                             run_coap(received_data_points_db, received_data_points_mqtt, command_state, command_state_lock),
                             run_mqtt(config, received_data_points_mqtt),
                             run_http_server(command_state, command_state_lock))
    except ValidationError as e:
        logging.critical("Failed to load config file")
        print(e)


if __name__ == "__main__":
    asyncio.run(main())
