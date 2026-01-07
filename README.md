# Q-Distributed-Database Client SDK

A high-performance, multi-language client SDK for [q-distributed-database](https://github.com/quannh0308/q-distributed-database), a distributed database system designed to compete with ScyllaDB and TiDB.

## Overview

This SDK provides a clean, type-safe interface for interacting with q-distributed-database, supporting:

- **CRUD Operations**: Full support for INSERT, SELECT, UPDATE, DELETE
- **Transactions**: ACID transactions with automatic rollback
- **Connection Pooling**: Efficient connection management with automatic failover
- **Query Builder**: Fluent API for building SQL queries safely
- **Authentication**: Secure authentication with token management
- **Admin Operations**: Cluster and user management
- **Multi-Language**: Implementations for Rust, Python, and TypeScript

## Features

- ✅ TCP/TLS protocol support with automatic protocol negotiation
- ✅ Bincode serialization with CRC32 checksums for data integrity
- ✅ Automatic retry with exponential backoff
- ✅ Connection pooling with health monitoring
- ✅ Streaming results for large datasets
- ✅ Property-based testing for correctness guarantees
- ✅ Comprehensive error handling
- ✅ Metrics and observability

## Project Status

🚧 **In Development** - This project is currently in the specification and design phase.

### Current Phase: Specification Complete

- ✅ Requirements document with 14 detailed requirements
- ✅ Design document with architecture and 42 correctness properties
- ✅ Implementation plan with 70+ tasks
- 🔄 Implementation in progress (Rust)

## Documentation

- [Requirements](/.kiro/specs/client-sdk/requirements.md) - Detailed requirements and acceptance criteria
- [Design](/.kiro/specs/client-sdk/design.md) - Architecture, components, and correctness properties
- [Tasks](/.kiro/specs/client-sdk/tasks.md) - Implementation plan and task breakdown

## Quick Start

*Coming soon - SDK is currently under development*

## Technical Specifications

- **Protocol**: TCP on port 7000 (default)
- **Serialization**: Bincode with CRC32 checksums
- **Message Framing**: Length-prefixed (4-byte big-endian)
- **Authentication**: Token-based with 24-hour TTL (default)
- **Connection Pool**: 5-20 connections (configurable)
- **Query Language**: Standard SQL with parameterized queries

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Client Application                       │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                   Client SDK (Public API)                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ Data Client  │  │ Admin Client │  │ Query Builder│     │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘     │
│         │                  │                  │              │
│         └──────────────────┴──────────────────┘              │
│                            │                                 │
│  ┌─────────────────────────▼──────────────────────────┐    │
│  │           Connection Manager & Pool                 │    │
│  └─────────────────────────┬──────────────────────────┘    │
│                            │                                 │
│  ┌─────────────────────────▼──────────────────────────┐    │
│  │         Protocol Layer (Message Codec)              │    │
│  └─────────────────────────┬──────────────────────────┘    │
└────────────────────────────┼────────────────────────────────┘
                             │
                             ▼ TCP/TLS (Port 7000)
┌─────────────────────────────────────────────────────────────┐
│              Q-Distributed-Database Cluster                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                  │
│  │  Node 1  │  │  Node 2  │  │  Node 3  │                  │
│  └──────────┘  └──────────┘  └──────────┘                  │
└─────────────────────────────────────────────────────────────┘
```

## Development

### Prerequisites

- Rust 1.70+ (for Rust implementation)
- Python 3.8+ (for Python implementation)
- Node.js 16+ (for TypeScript implementation)

### Building

*Coming soon*

### Testing

The SDK uses a dual testing approach:
- **Unit Tests**: Specific examples and edge cases
- **Property-Based Tests**: Universal correctness properties (100+ iterations per property)

### Contributing

This project follows spec-driven development:
1. Requirements define what to build
2. Design defines how to build it
3. Tasks break down the implementation
4. Property-based tests ensure correctness

## License

*To be determined*

## Acknowledgments

- Server implementation: [q-distributed-database](https://github.com/quannh0308/q-distributed-database)
- Inspired by ScyllaDB and TiDB client architectures
