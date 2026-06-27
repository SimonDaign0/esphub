# ESPHub

**ESPHub** is a personal project designed to bridge web-based controls with high-speed, low-power ESP-NOW communications. It establishes a central ESP32-C3 as a hosted Wi-Fi Access Point (AP), allowing users to send commands via HTTP and WebSockets, which the central hub then broadcasts to various ESP-NOW Station nodes.

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

> [Encryption]
> Web traffic to the Access Point (AP) is encrypted via WPA2, and ESP-NOW packets are secured using AES-CCM.
