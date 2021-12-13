import asyncio
import asyncpg
from collections import namedtuple
import json
import base64

QUERY = """
WITH
meta_table AS (
    select tx_id, json AS metadata from tx_metadata where key = '61284'
    ) ,
sig_table AS (
    select tx_id, json AS signature from tx_metadata where key = '61285'
    )
SELECT tx.hash, tx_id, metadata, signature, block_index
FROM meta_table
    INNER JOIN tx ON tx.id = meta_table.tx_id
    INNER JOIN sig_table USING(tx_id)
    INNER JOIN block ON block.id = tx.block_id
ORDER BY metadata -> '4' ASC;
"""


async def run():
    conn = await asyncpg.connect(user='postgres', password='postgres', port=5432,
                                 database='postgres', host='127.0.0.1')
    values = await conn.fetch(
        QUERY
    )
    await conn.close()
    return values

Transaction = namedtuple("Transaction", field_names=["tx_id", "metadata", "signature", "block_index"])
Metadata = namedtuple("Metadata", field_names=["voting_key", "stake_pub", "reward_address", "nonce"])


def entry_to_transaction(entry):
    metadata = json.loads(entry.get("metadata"))
    metadata = Metadata(
        voting_key=str(metadata.get("1")),
        stake_pub=str(metadata.get("2")),
        reward_address=str(metadata.get("3")),
        nonce=str(metadata.get("4"))
    )._asdict()
    return Transaction(
        tx_id=str(entry.get("tx_id")),
        metadata=metadata,
        signature=json.loads(entry.get("signature")),
        block_index=entry.get("block_index")
    )._asdict()


if __name__ == "__main__":
    transactions = list(map(entry_to_transaction, asyncio.run(run())))
    with open("addresses.json", "w", encoding="utf-8") as f:
        json.dump(transactions, f, indent=4)
