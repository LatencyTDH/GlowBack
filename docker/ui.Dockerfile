FROM python:3.11-slim

WORKDIR /app

ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1

COPY ui/requirements.txt /app/requirements.txt
RUN pip install --no-cache-dir -r /app/requirements.txt

COPY ui /app

RUN useradd -m -s /bin/false uiuser \
    && chown -R uiuser:uiuser /app

EXPOSE 8501

USER uiuser

CMD ["streamlit", "run", "app.py", "--server.address=0.0.0.0", "--server.port=8501"]
