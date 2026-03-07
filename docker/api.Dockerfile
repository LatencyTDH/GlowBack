FROM python:3.11-slim

WORKDIR /app

ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1

COPY api/requirements.txt /app/requirements.txt
RUN pip install --no-cache-dir -r /app/requirements.txt

COPY api/app /app/app

RUN useradd -m -s /bin/false apiuser \
    && mkdir -p /app/data \
    && chown -R apiuser:apiuser /app

EXPOSE 8000

VOLUME ["/app/data"]

USER apiuser

CMD ["uvicorn", "app.main:app", "--host", "0.0.0.0", "--port", "8000"]
