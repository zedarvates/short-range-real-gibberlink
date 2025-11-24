🔑 Access Badge Use Cases
1. Physical Access Control
• 	Entry into secure zones (labs, data centers, military bases, industrial sites).
• 	The badge displays a dynamic QR code validated by ultrasonic sync → impossible to copy or reuse.
• 	Optional human validation via PIN and checkboxes (authorize access, commands, diagnostics).

2. Authentication for Charging Stations
• 	Badge presented to an EV or drone charging station → charging only starts if validation succeeds.
• 	Restricts charging access to authorized users (fleets, staff, certified drones).

3. Mission Transmission for Drones
• 	Badge stores or displays a QR mission payload (patrol routes, control points, actions).
• 	Drone reads the QR + receives ultrasonic burst → mission securely validated and timestamped.
• 	Prevents unauthorized drones from receiving or executing commands.
•   Without avable connections

4. Quarantine and Filtering
• 	Badge can contain a permission list or status (authorized / unauthorized).
• 	If invalid → access denied, vehicle or drone quarantined.
• 	Useful to block unwanted devices in fleets or sensitive zones.

5. Audit and Traceability
• 	Badge with EEPROM/e‑paper active display logs passages: time, zone, validation.
• 	Logs are cryptographically signed → tamper-proof and verifiable later.
• 	Enables reconstruction of movements and usage in case of incidents.

6. Multi‑format Flexibility
• 	Badge can be implemented as:
• 	Paper card (static QR + ultrasonic validation).
• 	E‑paper card (dynamic QR + tactile PIN).
• 	Electronic ring (miniature QR + embedded microphone).
• 	Each format adapts to context: simple access, charging, mission transfer, or reinforced identity.

🛡️ Advantages
• 	Local security: no network dependency → resistant to remote attacks.
• 	Anti‑replay: QR + ultrasonic coupling, valid only for a few seconds.
• 	Human‑in‑the‑loop: PIN and checkboxes for critical authorizations.
• 	Auditability: signed logs, traceable, exportable.