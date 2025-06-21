### Simulation Engine.Core Capabilities

#### Discrete Event Simulation (DES): Essential for modeling the overall process flow.

Events like "AGV Arrives," "Robot Finishes Task," "Part Enters Cell," "MES Sends Order" trigger state changes and
subsequent actions.
This is fundamental for testing the logic of your MES/FES layer and understanding system throughput, bottlenecks, and
resource utilization.

#### Connectivity / Interoperability:

Communicate with your custom MES/FES layer (likely via APIs, OPC UA, MQTT, database connections, file exchange, or
custom protocols).
Potentially simulate or connect to simplified models of machine controllers.
Support future integration with PLC code (see below).

#### Extensibility / Customization

The ability to import your specific factory layout (CAD models), define custom robot models/tools,
model unique machine behaviors, and script complex logic is crucial.

#### Performance:

The engine needs to run simulations efficiently, ideally faster than real-time, to allow for rapid testing iterations of
your software changes.