FROM python:3.10-bullseye

WORKDIR /build/

COPY ./src/requirements.txt ./
RUN pip install -r requirements.txt

COPY ./src/ ./

ENTRYPOINT ["python3", "edge.py"]
