# BEP 46: Mutable Torrents

**Title**: Mutable Torrents  
**Version**: 1  
**Last-Modified**: 2014-04-22  
**Author**: Arvid Norberg  
**Status**: Draft  

---

## 1. Introduction

This document proposes a mechanism for mutable torrents, where the content of a torrent can be updated after its initial publication. This is achieved by using public-key cryptography to sign updates, ensuring that only the holder of the private key can modify the torrent's content.

---

## 2. Motivation

Traditional torrents are immutable; once a `.torrent` file is created, its content (e.g., file list, piece hashes) cannot be changed. This limitation makes it difficult to use BitTorrent for distributing dynamic content, such as software updates or frequently changing data.

Mutable torrents address this limitation by allowing updates to the torrent's content while maintaining data integrity and authenticity through cryptographic signatures.

---

## 3. Specification

### 3.1. Public and Private Keys

- Each mutable torrent is associated with a **public key** and a **private key**.
- The public key is used to verify updates, while the private key is used to sign updates.
- The public key is embedded in the torrent's metadata and distributed to all peers.
- The private key must be kept secure by the torrent's owner.

### 3.2. Torrent Metadata

The `.torrent` file for a mutable torrent includes the following additional fields:

- **`mutable`**: A boolean flag indicating that the torrent is mutable.
- **`public key`**: The public key used to verify updates.
- **`signature`**: A signature of the torrent's info dictionary, created using the private key.

### 3.3. Update Mechanism

- Updates to the torrent's content are distributed as **signed messages**.
- Each update includes:
  - The new content (e.g., updated file list, piece hashes).
  - A **timestamp** indicating when the update was created.
  - A **signature** of the update, created using the private key.
- Peers verify the signature using the public key before accepting the update.

### 3.4. Update Propagation

- Updates are propagated through the **Distributed Hash Table (DHT)**.
- Peers can query the DHT for the latest version of a mutable torrent using its public key.
- The DHT stores the most recent update, ensuring that all peers eventually converge on the latest version.

### 3.5. Conflict Resolution

- If multiple updates are published simultaneously, the update with the **latest timestamp** is considered valid.
- Peers discard updates with older timestamps to ensure consistency.

---

## 4. Security Considerations

- **Data Integrity**: Updates are signed using the private key, ensuring that only the owner can modify the torrent's content.
- **Replay Attacks**: The timestamp in each update prevents replay attacks, as older updates are ignored.
- **Key Management**: The private key must be securely stored to prevent unauthorized updates.

---

## 5. Compatibility

- Mutable torrents are compatible with traditional immutable torrents.
- Clients that do not support mutable torrents can still download the initial version of the torrent but will not receive updates.

---

## 6. Use Cases

- **Software Distribution**: Distribute software updates efficiently without requiring a central server.
- **News Feeds**: Publish frequently updated content, such as news articles or blog posts.
- **Data Feeds**: Distribute dynamic datasets, such as stock prices or weather data.

---

## 7. Implementation

- The implementation of mutable torrents requires support for:
  - Public-key cryptography (e.g., Ed25519).
  - Signed updates and timestamp validation.
  - DHT-based update propagation.

---

## 8. References

- **BEP 44**: Storage of Small Data in DHT (used for storing updates).
- **BEP 47**: Extensions to Mutable Torrents for more complex update mechanisms.

---

## 9. Conclusion

Mutable torrents provide a powerful mechanism for distributing dynamic content using the BitTorrent protocol. By leveraging public-key cryptography and the DHT, mutable torrents enable secure and efficient updates while maintaining compatibility with existing BitTorrent clients.

---