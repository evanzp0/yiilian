# BEP 44: Storage of Small Data in DHT

**Title**: Storage of Small Data in DHT  
**Version**: 1  
**Last-Modified**: 2013-09-25  
**Author**: Arvid Norberg  
**Status**: Draft  

---

## 1. Introduction

This document proposes a mechanism for storing small amounts of data in the BitTorrent Distributed Hash Table (DHT). The data is stored in a decentralized manner, allowing peers to store and retrieve small pieces of information without relying on a central server.

---

## 2. Motivation

The BitTorrent DHT is primarily used for storing `info hash` to `peer list` mappings. However, there are use cases where it would be beneficial to store other types of small data, such as:

- Mutable torrent updates (BEP 46).
- Peer exchange information.
- Metadata for decentralized applications.

This proposal extends the DHT to support the storage of small data items.

---

## 3. Specification

### 3.1. Data Structure

Each data item stored in the DHT consists of the following fields:

- **`v`**: The value of the data item. This is the actual data being stored.
- **`seq`**: A sequence number, which is a monotonically increasing integer. It is used to determine the most recent version of the data.
- **`sig`**: A signature of the data item, created using the private key corresponding to the public key used to store the data.
- **`cas`** (optional): A compare-and-swap value, used for conflict resolution.

### 3.2. Keys

- Data items are stored under a key, which is typically a hash of a public key (for mutable data) or a random value (for immutable data).
- For mutable data, the key is derived from the public key using a cryptographic hash function (e.g., SHA-1).

### 3.3. Operations

#### 3.3.1. `put` Operation

The `put` operation is used to store a data item in the DHT. It includes the following steps:

1. The data item is signed using the private key corresponding to the public key used to derive the key.
2. The data item is encoded in a bencoded format.
3. The `put` request is sent to the DHT nodes responsible for the key.

#### 3.3.2. `get` Operation

The `get` operation is used to retrieve a data item from the DHT. It includes the following steps:

1. The `get` request is sent to the DHT nodes responsible for the key.
2. The nodes return the most recent data item (based on the sequence number).
3. The data item is decoded and its signature is verified using the public key.

### 3.4. Conflict Resolution

- If multiple `put` operations are performed for the same key, the data item with the highest sequence number is considered the most recent.
- The optional `cas` field can be used to implement compare-and-swap semantics, ensuring that updates are only applied if the current value matches the expected value.

---

## 4. Security Considerations

- **Data Integrity**: The signature ensures that the data item has not been tampered with.
- **Replay Attacks**: The sequence number prevents replay attacks, as older data items are ignored.
- **Key Management**: The private key must be securely stored to prevent unauthorized modifications.

---

## 5. Use Cases

- **Mutable Torrents (BEP 46)**: Store updates to mutable torrents in the DHT.
- **Peer Exchange**: Store peer exchange information for decentralized peer discovery.
- **Decentralized Applications**: Store metadata or configuration data for decentralized applications.

---

## 6. Implementation

- The implementation of BEP 44 requires support for:
  - Signing and verifying data items using public-key cryptography.
  - Encoding and decoding data items in bencoded format.
  - Handling `put` and `get` operations in the DHT.

---

## 7. References

- **BEP 5**: DHT Protocol (base protocol for DHT operations).
- **BEP 46**: Mutable Torrents (uses BEP 44 for storing updates).

---

## 8. Conclusion

BEP 44 provides a mechanism for storing small amounts of data in the BitTorrent DHT. By leveraging public-key cryptography and sequence numbers, it ensures data integrity and supports use cases such as mutable torrents and decentralized applications.

---