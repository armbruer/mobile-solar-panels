FROM python:3.10-slim-bullseye

WORKDIR /build/
COPY ./src/ ./

RUN pip install -r requirements.txt

ENTRYPOINT ["python3", "edge.py"]
