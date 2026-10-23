# Fotos para COSMIC

Fotos es un navegador, visor y editor liviano de imágenes escrito en Rust con
libcosmic. Está pensado para integrarse con COSMIC Desktop y también puede abrir
una imagen o carpeta recibida desde la línea de comandos.

## Funciones

- Navegación de carpetas con vistas de grilla y lista, miniaturas y archivos ocultos opcionales.
- Visualización de PNG, JPEG, BMP, WebP, TIFF/TIF y el primer cuadro de GIF.
- Navegación entre las imágenes de una carpeta con botones o flechas del teclado.
- Zoom ajustado a ventana, ajustado al ancho, tamaño real y zoom manual.
- Zoom con `Ctrl` + rueda del mouse en pasos de 5 %.
- Recorte interactivo, rotación, reflejo y escalado con distintos filtros.
- Historial de transformaciones con varios pasos de deshacer mediante el menú o `Ctrl+Z`.
- Restablecer una imagen modificada desde su archivo original.
- Guardar una copia o sobrescribir después de confirmar cambios pendientes.
- Enviar archivos y carpetas a la papelera con confirmación.
- Borrado permanente con `Ctrl+Delete` y advertencia explícita.
- Información EXIF de cámara, captura, exposición, apertura, ISO, distancia focal y orientación.
- Aplicación automática de la orientación EXIF al visualizar y editar.
- Preferencias persistentes para vista, archivos ocultos y última carpeta utilizada.
- Caché persistente de miniaturas y precarga acotada de imágenes vecinas.
- Menú responsive en el visor, modo de pantalla completa y soporte de `F11`.

El historial guarda operaciones livianas y reconstruye la imagen desde el archivo
original al deshacer, evitando retener una copia completa por cada paso. El
historial se limpia después de guardar o restablecer.

## Requisitos

En Debian o Ubuntu se necesitan, como mínimo, las bibliotecas de desarrollo
utilizadas por libcosmic:

```bash
sudo apt install libxkbcommon-dev libfontconfig1-dev
```

También se requiere una instalación reciente de Rust y Cargo.

## Ejecutar

Abrir el navegador en la carpeta de imágenes predeterminada:

```bash
cargo run --release
```

Abrir directamente una imagen:

```bash
cargo run --release -- /ruta/a/foto.jpg
```

Abrir directamente una carpeta:

```bash
cargo run --release -- /ruta/a/imagenes
```

## Atajos principales

| Atajo | Acción |
| --- | --- |
| `←` / `→` | Imagen anterior o siguiente |
| `+` / `-` | Acercar o alejar |
| `0` | Tamaño real (100 %) |
| `Ctrl` + rueda | Cambiar zoom |
| Arrastrar con botón izquierdo | Desplazar una imagen ampliada |
| Botón central | Activar la herramienta de recorte |
| `Ctrl+O` | Abrir imagen |
| `Ctrl+S` | Guardar copia |
| `Ctrl+Z` | Deshacer última transformación |
| `Ctrl+1` / `Ctrl+2` | Cambiar entre vista de grilla y lista |
| `Ctrl+Espacio` | Mostrar información de la foto |
| `F1` | Mostrar información acerca de Fotos |
| `Ctrl+F1` | Mostrar los atajos de teclado |
| `Ctrl+W` / `Ctrl+Q` | Cerrar Fotos |
| `Delete` | Mover la selección a la papelera |
| `Ctrl+Delete` | Eliminar permanentemente |
| `F11` | Activar o salir de pantalla completa |
| `Esc` | Cerrar el panel, diálogo o herramienta abierta |
| `Backspace` | Volver al navegador desde el visor |

## Estado del proyecto

Fotos ya cubre el flujo diario de navegación, visualización y transformaciones
básicas. La primera creación de una miniatura todavía requiere decodificar la
imagen de origen; las aperturas posteriores reutilizan el caché persistente.
