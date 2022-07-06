import asyncio
import datetime
import logging
import threading

import toml
from pydantic import ValidationError

from model import CommandState, CommandTypes
from http_server import run_http_server
from mqtt import Config, run_mqtt
from coap import run_coap

logging.basicConfig(level=logging.DEBUG)


async def main():
    command_state = CommandState(CommandTypes.Nop, 0.0, 0.0, datetime.timezone.utc)
    command_state_lock = threading.Lock()
    received_data_points = asyncio.Queue()

    try:
        config_dict = toml.load("config.toml")
        config = Config.parse_obj(config_dict)
        await asyncio.gather(run_coap(received_data_points, command_state, command_state_lock),
                             run_mqtt(config, received_data_points),
                             run_http_server(command_state, command_state_lock))
    except ValidationError as e:
        logging.critical("Failed to load config file")
        print(e)


if __name__ == "__main__":
    asyncio.run(main())
