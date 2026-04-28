import asyncio
import logging
import parkeerservice


FORMAT = "%(levelname)s %(name)s %(asctime)-15s %(filename)s:%(lineno)d %(message)s"
logging.basicConfig(format=FORMAT)
logging.getLogger().setLevel(logging.INFO)


async def main():
    client = await parkeerservice.get_client()
    for permit in client.permits:
        print(permit)
    # sessions = await parkeerservice.get_sessions(client)
    # for session in sessions:
    #     print(session)


if __name__ == "__main__":
    asyncio.run(main())
