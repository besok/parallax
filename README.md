# Parallax 🌌

A digital twin and end-to-end testing framework for complex manufacturing execution systems.

## Overview

Parallax provides a complete, local simulation environment to develop, test,
and benchmark a Factory Systems.
It aims to replicate the entire factory terra, including the MES, FES systems, and all machinery—allowing for
robust testing in isolation without needing physical hardware or cloud dependencies.

## The Problem

Testing a distributed Factory systems is inherently difficult.
It communicates with numerous upstream and downstream systems (MES, FES, AGVs, AMRs, robotic cells, pertinent machinery,
etc.)
using various protocols.
Verifying its logic, performance, and correctness requires either a fully operational factory or
a complex, brittle set of mocks.
This slows down development, makes regression testing difficult,
and limits the ability to benchmark performance under specific, reproducible scenarios.

## Core Goals & Features

Parallax is designed to solve these problems by providing the following capabilities:

### 🎯 Isolated System Simulation

Run and test a single FES/MES instance completely locally. Parallax provides a set of tools that will significantly help
in creating a simulation layer for the surrounding ecosystem:

- MES Simulator: Mimics the Manufacturing Execution System, sending production orders over a simulated message bus.
- Machinery Simulators: Acts as virtual robotic cells and conveyors, providing mock endpoints for industrial protocols
  like OPC UA, HTTP, etc.
- AGV/AMR Simulators: Simulates the behavior of Automated Guided Vehicles and Autonomous Mobile Robots, allowing for
  testing of material transport logic.

### ⛓️ Multi-Instance Integration Testing

Create complex test scenarios involving multiple instances that communicate with each other.
This allows for testing distributed logic, load balancing, and failover strategies in a controlled, local environment.

### 📊 Performance Benchmarking

Generate and measure key performance indicators (KPIs) for the FES.
Track metrics like production order throughput,
task execution latency, and resource utilization to identify bottlenecks and validate optimizations.

### 🗺️ Data Flow Visualization

Create a **"data travel map"** for any given transaction.
Parallax will trace an event (e.g., a new production order) as it
flows through the MES, FES, and machinery, generating sequence diagrams to help understand the system's behavior and
debug issues.

### ✨ Real-time 3D Visualization

Parallax provides a dynamic 3D digital twin of the entire factory floor for a given test.
Watch robots, AGVs, and materials move in real-time according to the test logic, providing invaluable visual
feedback and a powerful demonstration tool.

----
The project is built with Rust 🦀 and is designed to be modular, extensible, and easy to use.
