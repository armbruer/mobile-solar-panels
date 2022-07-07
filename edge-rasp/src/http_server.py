import asyncio
import datetime
import logging
import socket
from ssl import SSLContext
from typing import Union, Optional, Callable, Type

import aiohttp.log
from aiohttp import web
from aiohttp.abc import AbstractAccessLogger
from aiohttp.web import HostSequence, _run_app
from aiohttp.web_app import Application
from aiohttp.web_log import AccessLogger
from aiohttp.web_request import Request
from typing_extensions import Awaitable

from model import CommandTypes, CommandState, DataPoint


async def update_command(app, command_type: CommandTypes, timeoffset=None, latitude=None, longitude=None):
    command_state = app['command_state']
    command_state_lock: asyncio.Lock = app['command_state_lock']
    logging.debug("HTTP: Acquiring lock...")
    await command_state_lock.acquire()
    command_state.command = command_type
    if latitude is not None and longitude is not None:
        local_timezone = datetime.timezone(offset=datetime.timedelta(minutes=timeoffset))
        command_state.set_location_command_data(local_timezone, latitude, longitude)
    command_state_lock.release()
    logging.debug("HTTP: Lock released")


async def location(request: Request):
    data = await request.json()
    await update_command(request.app, CommandTypes.Location, data['timeoffset'], data['latitude'], data['longitude'])
    return web.Response()


async def light_tracking(request: Request):
    await update_command(request.app, CommandTypes.LightTracking)
    return web.Response()


async def stop(request: Request):
    await update_command(request.app, CommandTypes.Stop)
    return web.Response()


async def control(_request: Request):
    return web.FileResponse("control.html")


async def generate_data(received_data_points: asyncio.Queue):
    logging.warning("Creating mock values")
    while True:
        dp = DataPoint(datetime.datetime.utcnow().replace(tzinfo=datetime.timezone.utc),
                       8651.1876, 588, 9911, 2435, 456767, 93454)
        await received_data_points.put([dp])
        logging.info("put some data")
        await asyncio.sleep(1)


async def run_app(
    app: Union[Application, Awaitable[Application]],
    *,
    host: Optional[Union[str, HostSequence]] = None,
    port: Optional[int] = None,
    path: Optional[str] = None,
    sock: Optional[socket.socket] = None,
    shutdown_timeout: float = 60.0,
    keepalive_timeout: float = 75.0,
    ssl_context: Optional[SSLContext] = None,
    print: Callable[..., None] = print,
    backlog: int = 128,
    access_log_class: Type[AbstractAccessLogger] = AccessLogger,
    access_log_format: str = AccessLogger.LOG_FORMAT,
    access_log: Optional[logging.Logger] = aiohttp.log.access_logger,
    handle_signals: bool = True,
    reuse_address: Optional[bool] = None,
    reuse_port: Optional[bool] = None,
    loop: Optional[asyncio.AbstractEventLoop] = None,
) -> None:
    """Run an app locally"""
    if loop is None:
        loop = asyncio.get_running_loop()

    # Configure if and only if in debugging mode and using the default logger
    if loop.get_debug() and access_log and access_log.name == "aiohttp.access":
        if access_log.level == logging.NOTSET:
            access_log.setLevel(logging.DEBUG)
        if not access_log.hasHandlers():
            access_log.addHandler(logging.StreamHandler())

    main_task = loop.create_task(
        _run_app(
            app,
            host=host,
            port=port,
            path=path,
            sock=sock,
            shutdown_timeout=shutdown_timeout,
            keepalive_timeout=keepalive_timeout,
            ssl_context=ssl_context,
            print=print,
            backlog=backlog,
            access_log_class=access_log_class,
            access_log_format=access_log_format,
            access_log=access_log,
            handle_signals=handle_signals,
            reuse_address=reuse_address,
            reuse_port=reuse_port,
        )
    )

    await main_task


async def run_http_server(command_state: CommandState, command_state_lock: asyncio.Lock):
    app = web.Application()
    app['command_state'] = command_state
    app['command_state_lock'] = command_state_lock
    app.add_routes([web.post('/api/v1/location', location)])
    app.add_routes([web.post('/api/v1/light_tracking', light_tracking)])
    app.add_routes([web.post('/api/v1/stop', stop)])
    app.add_routes([web.get('/', control)])
    await run_app(app)
