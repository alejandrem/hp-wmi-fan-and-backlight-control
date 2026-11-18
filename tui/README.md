# Ale Top

Monitor de sistema para Linux, escrito en Rust y con una interfaz de terminal inspirada en **btop**. Muestra uso de CPU y núcleos, memoria, discos, red, procesos, batería, temperaturas, ventiladores y consumo cuando el hardware expone esas lecturas.

El proyecto también conserva el binario `fanctl` para controlar ventiladores HP compatibles; Ale Top es solo de monitoreo y no cambia la configuración del equipo.

## Requisitos

- Linux con `/proc` y `/sys` montados.
- Rust estable (edition 2024).
- Terminal compatible con crossterm para la interfaz interactiva.
- Opcional: `nvidia-smi` para lecturas de GPU NVIDIA y `ip` para mostrar la dirección IPv4.

## Compilar y ejecutar

Desde este directorio:

```sh
cargo build --release --bin aletop
cargo run --release --bin aletop
```

El ejecutable queda en `target/release/aletop`. En una terminal, presiona `q` o `Esc` para salir. Si la salida no es una terminal, Ale Top imprime un resumen de las lecturas disponibles.

En el panel de ventiladores puedes hacer clic en `AUTO`, `MAX`, ajustar RPM con `−`/`+` y aplicar hasta 6000 RPM. Los atajos son `F1` (AUTO), `F2` (MAX) y `F3` para escribir RPM y confirmar con `Enter`. Al pulsar un modo aparece inmediatamente un modal de progreso, que luego muestra el resultado. Si `sudo` necesita autenticación, la contraseña se captura oculta dentro de ese modal.

Para compilar la herramienta de ventiladores por separado:

```sh
cargo build --release --bin fanctl
```

`fanctl` controla laptops HP compatibles con Linux y el módulo `hp-wmi` (para RPM exactos, el driver debe exponer `fan1_target` y `fan2_target`):

```sh
sudo ./target/release/fanctl status
sudo ./target/release/fanctl rpm 3000
sudo ./target/release/fanctl auto
sudo ./target/release/fanctl max
```

Los cambios manuales pueden tardar varios segundos en reflejarse. Usa `auto` para devolver el control al firmware.

## Sensores y compatibilidad

Los datos generales se leen de `/proc` y `/sys`. Las temperaturas y RPM se buscan entre los dispositivos hwmon disponibles, así que no dependen del número variable `hwmonN`. La disponibilidad y los nombres de sensores dependen del kernel, los controladores y el hardware; las lecturas ausentes se muestran como `N/A` o se omiten.

La lista de procesos se ordena por actividad de CPU desde la lectura anterior. En la primera actualización todavía no hay una lectura previa, por lo que el uso de CPU de los procesos aparece en cero.

## Estado del proyecto

Ale Top es un monitor inspirado en btop, no un clon completo: todavía no incluye toda la configuración, navegación, filtros ni personalización de btop.
