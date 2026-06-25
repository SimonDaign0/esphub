# ESPHub

**ESPHub** is a personal project designed to bridge web-based controls with high-speed, low-power ESP-NOW communications. It establishes a central ESP32-C3 as a Wi-Fi Access Point (AP), allowing users to send commands via HTTP and WebSockets, which the central hub then broadcasts to various ESP-NOW Station nodes.

## Architecture Overview

```
[ User Device ]
       │
       ▼ (HTTP / WebSockets)
 [ ESP32-C3 Hub (AP) ]
       │
       ▼ (ESP-NOW)
[ Station 1 ] [ Station 2 ] [ Station N ]
```

> [!IMPORTANT]
> **Security Notice:** As of the current version, **no data encryption** has been implemented. This project is currently intended for development and testing.
