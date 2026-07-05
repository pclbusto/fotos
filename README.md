# Visor de Fotos COSMIC (libcosmic)

Este es un visor de fotos moderno, elegante e interactivo hecho en **Rust** utilizando el toolkit **libcosmic** (del entorno de escritorio COSMIC Desktop por System76).

## Características

- 🖼️ **Cargar y visualizar imágenes**: Carga nativa de imágenes estáticas (PNG, JPEG, BMP, WebP y TIFF/TIF) y primer frame de archivos GIF mediante diálogo de archivos integrado.
- 🔍 **Control de Zoom**:
  - Acercar (+).
  - Alejar (-).
  - Ver a tamaño real (100%).
- 🖐️ **Modo Navegación (Paneo)**: Arrastra la imagen libremente con el mouse para explorar áreas detalladas cuando esté ampliada.
- ✂️ **Modo Recorte Interactivo (Crop)**:
  - Selecciona de forma interactiva la región a recortar arrastrando el mouse sobre la imagen.
  - Cuenta con una máscara de oscurecimiento (sombras) alrededor del área seleccionada para una excelente experiencia de usuario.
  - Botón para confirmar el recorte en el visor (`Confirmar Recorte`).
- 🔄 **Restablecer**: Botón para restaurar la imagen original del archivo en caso de querer descartar los recortes actuales.
- 💾 **Guardar Copia**: Guarda la imagen resultante (con o sin recortes) como una copia nueva a través de un diálogo nativo de guardado de archivos.
- 📊 **Barra de Estado**: Muestra el nombre de archivo abierto, dimensiones reales del lienzo y el porcentaje de zoom actual.

## Requisitos Previos

Dado que `libcosmic` se basa en `iced` y se integra de forma nativa con el sistema operativo, asegúrate de tener las bibliotecas de desarrollo del sistema necesarias (como `libxkbcommon-dev` y `libfontconfig-dev` en distribuciones Debian/Ubuntu):

```bash
sudo apt install libxkbcommon-dev libfontconfig1-dev
```

## Cómo Ejecutar el Proyecto

Para compilar y ejecutar el visor de fotos, simplemente abre una terminal en el directorio del proyecto y ejecuta:

```bash
cargo run --release
```

## Instrucciones de Uso

1. Haz clic en **Abrir Imagen** para cargar una foto desde tu disco.
2. Utiliza los botones de **Acercar**, **Alejar** o **100%** para ajustar el zoom.
3. **Mover la imagen**: Asegúrate de estar en el modo `Navegar/Mover` (por defecto) y arrastra la imagen con el clic izquierdo del mouse para desplazarla.
4. **Recortar la imagen**:
   - Cambia al modo de recorte haciendo clic en el botón de modo (el botón mostrará `Modo: Seleccionar Recorte`).
   - Haz clic izquierdo y arrastra sobre la imagen para seleccionar el área que deseas recortar (verás el rectángulo blanco con el fondo oscurecido alrededor).
   - Presiona el botón **Confirmar Recorte** que aparecerá en la barra de herramientas para aplicar el recorte en pantalla.
   - Si no te gusta el resultado, puedes hacer clic en **Restablecer** para volver a la imagen original.
5. Haz clic en **Guardar Copia** para guardar el recorte o la imagen actual en el formato que desees (PNG o JPEG).
