### Simulation Engine.Core Capabilities
#### Discrete Event Simulation (DES): Essential for modeling the overall process flow.
Events like "AGV Arrives," "Robot Finishes Task," "Part Enters Cell," "MES Sends Order" trigger state changes and subsequent actions.
This is fundamental for testing the logic of your MES/FES layer and understanding system throughput, bottlenecks, and resource utilization.

#### Physics Simulation:

##### Kinematics:
Accurately simulating the movement of robot arms (joint angles, end-effector position/orientation) based on commands.
This is crucial for reachability analysis, basic path validation, and cycle time estimation.

##### Collision Detection:
Detecting interference between robots, machinery, AGVs, parts, and the environment.
Essential for validating generated paths and preventing physical crashes.

##### (Optional) Dynamics: Simulating forces, torques, inertia, and gravity.
This offers higher fidelity, potentially needed for very precise tasks, force-controlled operations,
or stability analysis, but significantly increases computational load.
For initial testing of your calculation layer, accurate kinematics and collision detection might be sufficient.

#### Agent-Based Modeling (for AGVs):
Simulating the individual behavior and decision-making of each AGV within the fleet,
including pathfinding, traffic management (collision avoidance, deadlocks),
charging strategies, and task allocation based on requests from your system.

#### Connectivity / Interoperability:
Communicate with your custom MES/FES layer (likely via APIs, OPC UA, MQTT, database connections, file exchange, or custom protocols).
Potentially simulate or connect to simplified models of machine controllers.
Support future integration with PLC code (see below).

#### Extensibility / Customization
The ability to import your specific factory layout (CAD models), define custom robot models/tools,
model unique machine behaviors, and script complex logic is crucial.

#### Performance:
The engine needs to run simulations efficiently, ideally faster than real-time, to allow for rapid testing iterations of your software changes.