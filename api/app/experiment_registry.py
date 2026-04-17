from __future__ import annotations

import json
import sqlite3
from dataclasses import asdict, is_dataclass
from datetime import date, datetime
from functools import lru_cache
from hashlib import sha256
from pathlib import Path
from threading import RLock
from typing import Any


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def default_registry_path() -> Path:
    import os

    env_value = os.getenv("GLOWBACK_REGISTRY_PATH")
    if env_value:
        return Path(env_value).expanduser().resolve()
    return (_repo_root() / "data" / "experiment-registry.sqlite3").resolve()


def sha256_text(value: str) -> str:
    return sha256(value.encode("utf-8")).hexdigest()


def _to_jsonable(value: Any) -> Any:
    if value is None:
        return None
    if hasattr(value, "model_dump"):
        return _to_jsonable(value.model_dump(mode="json"))
    if is_dataclass(value):
        return _to_jsonable(asdict(value))
    if isinstance(value, dict):
        return {str(key): _to_jsonable(item) for key, item in value.items()}
    if isinstance(value, (list, tuple, set)):
        return [_to_jsonable(item) for item in value]
    if isinstance(value, (datetime, date)):
        return value.isoformat()
    return value


def stable_json_dumps(value: Any) -> str:
    return json.dumps(_to_jsonable(value), sort_keys=True, separators=(",", ":"), ensure_ascii=False)


class ExperimentRegistry:
    def __init__(self, path: str | Path | None = None) -> None:
        self.path = Path(path).expanduser().resolve() if path else default_registry_path()
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self._lock = RLock()
        self._conn = sqlite3.connect(self.path, check_same_thread=False)
        self._conn.row_factory = sqlite3.Row
        self._ensure_schema()

    def _ensure_schema(self) -> None:
        with self._lock, self._conn:
            self._conn.executescript(
                """
                CREATE TABLE IF NOT EXISTS runs (
                    run_id TEXT PRIMARY KEY,
                    source TEXT NOT NULL,
                    state TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    deleted_at TEXT,
                    label TEXT,
                    status_json TEXT NOT NULL,
                    request_json TEXT,
                    result_json TEXT,
                    events_json TEXT NOT NULL DEFAULT '[]',
                    metadata_json TEXT NOT NULL DEFAULT '{}',
                    strategy_name TEXT,
                    strategy_code TEXT,
                    strategy_code_hash TEXT,
                    strategy_config_json TEXT
                );

                CREATE INDEX IF NOT EXISTS idx_runs_source_created
                    ON runs(source, created_at DESC);
                CREATE INDEX IF NOT EXISTS idx_runs_state_created
                    ON runs(state, created_at DESC);

                CREATE TABLE IF NOT EXISTS strategies (
                    name TEXT PRIMARY KEY,
                    code TEXT NOT NULL,
                    code_hash TEXT NOT NULL,
                    config_json TEXT NOT NULL,
                    metadata_json TEXT NOT NULL DEFAULT '{}',
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    deleted_at TEXT
                );

                CREATE INDEX IF NOT EXISTS idx_strategies_updated
                    ON strategies(updated_at DESC);
                """
            )

    def close(self) -> None:
        with self._lock:
            self._conn.close()

    def _fetch_existing(self, table: str, key_column: str, key_value: str) -> sqlite3.Row | None:
        cursor = self._conn.execute(
            f"SELECT * FROM {table} WHERE {key_column} = ?",
            (key_value,),
        )
        return cursor.fetchone()

    def upsert_run(
        self,
        *,
        run_id: str,
        source: str,
        status: Any,
        request: Any | None = None,
        result: Any | None = None,
        events: Any | None = None,
        metadata: Any | None = None,
        strategy_name: str | None = None,
        strategy_code: str | None = None,
        strategy_config: Any | None = None,
        label: str | None = None,
    ) -> dict[str, Any]:
        now = datetime.utcnow().isoformat() + "Z"
        status_payload = _to_jsonable(status) or {}
        request_payload = _to_jsonable(request)
        result_payload = _to_jsonable(result)
        events_payload = _to_jsonable(events) or []
        metadata_payload = _to_jsonable(metadata) or {}
        strategy_config_payload = _to_jsonable(strategy_config) or {}

        created_at = status_payload.get("created_at") or now
        state = status_payload.get("state") or "unknown"
        strategy_code_hash = sha256_text(strategy_code) if strategy_code else None

        with self._lock, self._conn:
            existing = self._fetch_existing("runs", "run_id", run_id)
            if existing is not None:
                created_at = existing["created_at"]
                if label is None:
                    label = existing["label"]
                if strategy_name is None:
                    strategy_name = existing["strategy_name"]
                if strategy_code is None:
                    strategy_code = existing["strategy_code"]
                    strategy_code_hash = existing["strategy_code_hash"]
                if not strategy_config_payload:
                    strategy_config_payload = json.loads(existing["strategy_config_json"] or "{}")
                if request_payload is None and existing["request_json"]:
                    request_payload = json.loads(existing["request_json"])
                if result_payload is None and existing["result_json"]:
                    result_payload = json.loads(existing["result_json"])
                if (not events_payload) and existing["events_json"]:
                    events_payload = json.loads(existing["events_json"])
                if (not metadata_payload) and existing["metadata_json"]:
                    metadata_payload = json.loads(existing["metadata_json"])

            self._conn.execute(
                """
                INSERT INTO runs (
                    run_id,
                    source,
                    state,
                    created_at,
                    updated_at,
                    deleted_at,
                    label,
                    status_json,
                    request_json,
                    result_json,
                    events_json,
                    metadata_json,
                    strategy_name,
                    strategy_code,
                    strategy_code_hash,
                    strategy_config_json
                ) VALUES (?, ?, ?, ?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(run_id) DO UPDATE SET
                    source = excluded.source,
                    state = excluded.state,
                    updated_at = excluded.updated_at,
                    deleted_at = NULL,
                    label = excluded.label,
                    status_json = excluded.status_json,
                    request_json = excluded.request_json,
                    result_json = excluded.result_json,
                    events_json = excluded.events_json,
                    metadata_json = excluded.metadata_json,
                    strategy_name = excluded.strategy_name,
                    strategy_code = excluded.strategy_code,
                    strategy_code_hash = excluded.strategy_code_hash,
                    strategy_config_json = excluded.strategy_config_json
                """,
                (
                    run_id,
                    source,
                    state,
                    created_at,
                    now,
                    label,
                    stable_json_dumps(status_payload),
                    stable_json_dumps(request_payload) if request_payload is not None else None,
                    stable_json_dumps(result_payload) if result_payload is not None else None,
                    stable_json_dumps(events_payload),
                    stable_json_dumps(metadata_payload),
                    strategy_name,
                    strategy_code,
                    strategy_code_hash,
                    stable_json_dumps(strategy_config_payload),
                ),
            )

        record = self.get_run(run_id)
        if record is None:  # pragma: no cover - defensive only
            raise RuntimeError(f"Failed to persist run {run_id}")
        return record

    def list_runs(
        self,
        *,
        source: str | None = None,
        state: str | None = None,
        limit: int = 200,
        include_deleted: bool = False,
    ) -> list[dict[str, Any]]:
        query = "SELECT * FROM runs"
        conditions: list[str] = []
        params: list[Any] = []

        if not include_deleted:
            conditions.append("deleted_at IS NULL")
        if source:
            conditions.append("source = ?")
            params.append(source)
        if state:
            conditions.append("state = ?")
            params.append(state)

        if conditions:
            query += " WHERE " + " AND ".join(conditions)
        query += " ORDER BY created_at DESC LIMIT ?"
        params.append(limit)

        with self._lock:
            rows = self._conn.execute(query, params).fetchall()
        return [self._row_to_run(row) for row in rows]

    def get_run(self, run_id: str, *, include_deleted: bool = False) -> dict[str, Any] | None:
        query = "SELECT * FROM runs WHERE run_id = ?"
        params: list[Any] = [run_id]
        if not include_deleted:
            query += " AND deleted_at IS NULL"

        with self._lock:
            row = self._conn.execute(query, params).fetchone()
        return self._row_to_run(row) if row else None

    def rename_run(self, run_id: str, label: str | None) -> dict[str, Any] | None:
        now = datetime.utcnow().isoformat() + "Z"
        with self._lock, self._conn:
            self._conn.execute(
                "UPDATE runs SET label = ?, updated_at = ?, deleted_at = NULL WHERE run_id = ?",
                (label, now, run_id),
            )
        return self.get_run(run_id)

    def delete_run(self, run_id: str) -> bool:
        now = datetime.utcnow().isoformat() + "Z"
        with self._lock, self._conn:
            cursor = self._conn.execute(
                "UPDATE runs SET deleted_at = ?, updated_at = ? WHERE run_id = ? AND deleted_at IS NULL",
                (now, now, run_id),
            )
        return cursor.rowcount > 0

    def upsert_strategy(
        self,
        *,
        name: str,
        code: str,
        config: Any | None = None,
        metadata: Any | None = None,
    ) -> dict[str, Any]:
        now = datetime.utcnow().isoformat() + "Z"
        config_payload = _to_jsonable(config) or {}
        metadata_payload = _to_jsonable(metadata) or {}
        code_hash = sha256_text(code)

        with self._lock, self._conn:
            existing = self._fetch_existing("strategies", "name", name)
            created_at = existing["created_at"] if existing is not None else now
            self._conn.execute(
                """
                INSERT INTO strategies (
                    name,
                    code,
                    code_hash,
                    config_json,
                    metadata_json,
                    created_at,
                    updated_at,
                    deleted_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, NULL)
                ON CONFLICT(name) DO UPDATE SET
                    code = excluded.code,
                    code_hash = excluded.code_hash,
                    config_json = excluded.config_json,
                    metadata_json = excluded.metadata_json,
                    updated_at = excluded.updated_at,
                    deleted_at = NULL
                """,
                (
                    name,
                    code,
                    code_hash,
                    stable_json_dumps(config_payload),
                    stable_json_dumps(metadata_payload),
                    created_at,
                    now,
                ),
            )
        record = self.get_strategy(name)
        if record is None:  # pragma: no cover - defensive only
            raise RuntimeError(f"Failed to persist strategy {name}")
        return record

    def list_strategies(self, *, include_deleted: bool = False, limit: int = 200) -> list[dict[str, Any]]:
        query = "SELECT * FROM strategies"
        params: list[Any] = []
        if not include_deleted:
            query += " WHERE deleted_at IS NULL"
        query += " ORDER BY updated_at DESC LIMIT ?"
        params.append(limit)
        with self._lock:
            rows = self._conn.execute(query, params).fetchall()
        return [self._row_to_strategy(row) for row in rows]

    def get_strategy(self, name: str, *, include_deleted: bool = False) -> dict[str, Any] | None:
        query = "SELECT * FROM strategies WHERE name = ?"
        params: list[Any] = [name]
        if not include_deleted:
            query += " AND deleted_at IS NULL"
        with self._lock:
            row = self._conn.execute(query, params).fetchone()
        return self._row_to_strategy(row) if row else None

    def delete_strategy(self, name: str) -> bool:
        now = datetime.utcnow().isoformat() + "Z"
        with self._lock, self._conn:
            cursor = self._conn.execute(
                "UPDATE strategies SET deleted_at = ?, updated_at = ? WHERE name = ? AND deleted_at IS NULL",
                (now, now, name),
            )
        return cursor.rowcount > 0

    def _row_to_run(self, row: sqlite3.Row) -> dict[str, Any]:
        return {
            "run_id": row["run_id"],
            "source": row["source"],
            "state": row["state"],
            "created_at": row["created_at"],
            "updated_at": row["updated_at"],
            "deleted_at": row["deleted_at"],
            "label": row["label"],
            "status": json.loads(row["status_json"]),
            "request": json.loads(row["request_json"]) if row["request_json"] else None,
            "result": json.loads(row["result_json"]) if row["result_json"] else None,
            "events": json.loads(row["events_json"] or "[]"),
            "metadata": json.loads(row["metadata_json"] or "{}"),
            "strategy_name": row["strategy_name"],
            "strategy_code": row["strategy_code"],
            "strategy_code_hash": row["strategy_code_hash"],
            "strategy_config": json.loads(row["strategy_config_json"] or "{}"),
        }

    def _row_to_strategy(self, row: sqlite3.Row) -> dict[str, Any]:
        return {
            "name": row["name"],
            "code": row["code"],
            "code_hash": row["code_hash"],
            "config": json.loads(row["config_json"] or "{}"),
            "metadata": json.loads(row["metadata_json"] or "{}"),
            "created_at": row["created_at"],
            "updated_at": row["updated_at"],
            "deleted_at": row["deleted_at"],
        }


@lru_cache(maxsize=1)
def get_default_registry() -> ExperimentRegistry:
    return ExperimentRegistry()
