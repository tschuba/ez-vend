---
title: Specification
nav_order: 2
parent: Redesign
---

# ez-vend-rs

---
**Document Status:** Historical Reference
**Last Updated:** 2026-04-04
**Purpose:** Original redesign specification capturing the initial scope and goals of the Rust/WASM project.

---

## Introduction

`ez-vend-rs` is a redesign of `ez-booth` in Rust.  
The goal of this project is to create a more efficient, less resource-intense and robust version of `ez-booth` while maintaining its ease of use and functionality.  
The concept is to migrate to a WebAssembly (WASM) based architecture, which allows for better performance and cross-platform compatibility.  
This redesign shall also focus on improving the user experience, making it more intuitive and user-friendly.  

## Core features

The redesign shall retain the core features of `ez-booth`, including:

- Portable application without any need for installation
- Offline functionality, allowing users to use the application without an internet connection
- Parallel run of one or more disconnected instances
- optional sync feature using file transfer or wireless or wired network connection if available to consolidate data from multiple instances and bring all clients to the same state
- User-friendly interface for easy navigation and operation running in any of a client's provided web browser
- Report printing capabilities, allowing users to generate and print reports based on the data collected by the application

## Extended features

- Support for various deployment options
    1. Backend and frontend running on the same machine (local deployment)
    2. Backend running on a server and frontend running on clients (client-server deployment)
- Enhanced data management and synchronization features, allowing for better handling of data across multiple instances and clients

## Implementation Steps

1. Analysis of current state of `ez-booth`
2. Design of the new architecture and user interface
3. Identification of areas for improvement
4. Specification of implementation details and requirements
5. Review iterations of design and final architecture approval
6. Development of the new application using Rust and WebAssembly
    1. Setting up the development environment and tools
    2. Implementing the core features and functionalities
    3. Integrating the new architecture and ensuring compatibility with existing features
    4. Adding extended features and functionalities as needed
7. Testing and debugging to ensure functionality and performance
8. Optimization and performance improvements based on testing results
9. Documentation and user guide creation
10. Release and deployment of the new application
