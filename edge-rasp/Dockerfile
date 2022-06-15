FROM python:3.10-slim-bullseye

WORKDIR /build/
COPY ./ ./

RUN pip install -r requirements.txt

WORKDIR /build/src/
ENTRYPOINT ["python3", "edge.py"]
