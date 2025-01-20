# BEP: 11  
**Title:** Peer Exchange (PEX)  
**Version:** 1.0  
**Last-Modified:** 2005-05-02  
**Author:** Arvid Norberg  
**Status:** Draft  

## Abstract

This document describes an extension to the BitTorrent peer protocol that allows peers to exchange information about other peers. This extension is known as Peer Exchange (PEX).

## 1. Introduction

Peer Exchange (PEX) is a mechanism for peers to inform each other about the existence of additional peers in the swarm. This can be used to speed up the process of finding peers, especially in situations where the tracker is slow or unresponsive.

## 2. Protocol

The PEX extension is implemented using the extension protocol defined in BEP-10. The extension message for PEX is identified by the extension ID `ut_pex`.

### 2.1. Extension Handshake

During the extension handshake, peers indicate support for PEX by including the `ut_pex` extension ID in the `m` dictionary of the handshake message.

Example:

```plaintext
{
  "m": {
    "ut_pex": 1
  }
}
```

### 2.2. PEX Message Format

The PEX message is a bencoded dictionary with the following fields:

- **added**: A string containing compact IP/port information for newly added peers.
- **added.f**: A string containing flags for the newly added peers (optional).
- **dropped**: A string containing compact IP/port information for peers that have been dropped (optional).

Example:

```plaintext
{
  "added": "<compact peer info>",
  "added.f": "<flags>",
  "dropped": "<compact peer info>"
}
```

### 2.3. Compact Peer Information

The compact peer information is a string where each peer is represented by 6 bytes:

- **4 bytes** for the IP address (network byte order).
- **2 bytes** for the port number (network byte order).

### 2.4. Flags

The flags field is a string where each byte represents the flags for a corresponding peer in the `added` field. The flags are defined as follows:

- **Bit 0**: Peer supports the DHT protocol.
- **Bit 1**: Peer supports the Fast extension.

## 3. Behavior

- Peers **SHOULD** send PEX messages periodically to inform each other about new and dropped peers.
- Peers **SHOULD** limit the rate at which they send PEX messages to avoid excessive bandwidth usage.
- Peers **SHOULD NOT** send PEX messages to peers that have not indicated support for the PEX extension.

## 4. Security Considerations

- Peers **SHOULD** validate the IP addresses received in PEX messages to prevent spoofing.
- Peers **SHOULD** limit the number of peers they add from PEX messages to avoid being overwhelmed by malicious peers.

## 5. References

- **[BEP-3]**: The BitTorrent Protocol Specification  
- **[BEP-10]**: Extension Protocol  
- **[BEP-5]**: DHT Protocol  