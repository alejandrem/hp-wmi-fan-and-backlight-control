# fanctl — Control de ventiladores HP Victus (CLI en Rust)

Herramienta ligera de línea de comandos para controlar los ventiladores de laptops HP Victus en Linux. Escrita en Rust puro, sin dependencias externas.

## Requisitos

- Driver `hp-wmi` cargado (stock, TUXOV, o hp-wmi-extended)
- El módulo debe exponer `fan1_target` y `fan2_target` (hp-wmi-extended recomendado)
- Rust toolchain instalado

## Compilación

```bash
cd tui
cargo build --release
```

El binario queda en `target/release/fanctl`.

## Instalación

```bash
# Copiar el binario a una ruta en $PATH
sudo cp target/release/fanctl /usr/local/bin/fanctl

# O instalar con cargo
cargo install --path .
```

## Uso

```bash
sudo ./tui/target/release/fanctl rpm 3000      # 3000 RPM exactos
sudo ./tui/target/release/fanctl rpm 5000      # 5000 RPM (por encima del límite BIOS)
sudo ./tui/target/release/fanctl rpm 6000      # Máximo hardware (~6000 RPM)
sudo ./tui/target/release/fanctl auto          # Control automático del BIOS
sudo ./tui/target/release/fanctl max           # Ventiladores al máximo fijo
sudo ./tui/target/release/fanctl status        # Ver estado actual
```

## Comandos

| Comando | Descripción |
|---------|-------------|
| `sudo fanctl rpm [0-6500]` | Establece velocidad exacta en RPM. Activa modo MANUAL automáticamente. |
| `sudo fanctl auto` | Devuelve el control al BIOS/firmware. |
| `sudo fanctl max` | Ventiladores al 100% fijo. |
| `sudo fanctl status` | Muestra modo actual, RPM de ambos ventiladores, targets y límites del BIOS. |

## Notas

- El EC (Embedded Controller) tarda ~10 segundos en responder a cambios de RPM. El comando `rpm` espera automáticamente antes de mostrar el resultado.
- El keep-alive del driver mantiene la velocidad configurada cada 90 segundos.
- El hardware clampa los valores por encima de ~6000 RPM.

## Compatibilidad

Testeado en:
- HP Victus 15-fb2xxx (board 8C2F)
- Kernel Linux 7.3.4-1-cachyos
- Driver hp-wmi-extended con soporte `fan_target`
- El modo max está diseñado para ser un "turbo" rápido del BIOS, no para
control granular. Si quieres el máximo real del hardware (6000 RPM), usa siempre rpm 6000.

## Licencia

Misma licencia que el repositorio padre: [MIT](../LICENSE)
