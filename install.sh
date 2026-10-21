#!/usr/bin/env bash
set -euo pipefail

APP_ID="com.guten.Fotos"
PROJECT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DESKTOP_SOURCE="$PROJECT_DIR/$APP_ID.desktop"
DEFAULT_ICON_SOURCE="$PROJECT_DIR/fotos-icono.png"
ICON_SOURCE="$DEFAULT_ICON_SOURCE"
RELEASE_BIN="$PROJECT_DIR/target/release/visor_fotos"
BIN_DIR="${HOME}/.local/bin"
APPLICATIONS_DIR="${XDG_DATA_HOME:-${HOME}/.local/share}/applications"
ICON_ROOT="${XDG_DATA_HOME:-${HOME}/.local/share}/icons/hicolor"
INSTALLED_BIN="$BIN_DIR/visor_fotos"
INSTALLED_DESKTOP="$APPLICATIONS_DIR/$APP_ID.desktop"

usage() {
    cat <<'EOF'
Uso: ./install.sh [opción] [imagen]

Opciones:
  --full,  -f   Compila en modo release e instala todo (predeterminado).
  --copy,  -c   Instala usando el binario release ya compilado.
  --icons, -i   Escala e instala una imagen y actualiza la caché.
  --help,  -h   Muestra esta ayuda.

Ejemplos:
  ./install.sh
  ./install.sh --full
  ./install.sh --copy
  ./install.sh --icons
  ./install.sh --icons ./otro-icono.png
EOF
}

require_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "Falta el comando requerido: $1" >&2
        exit 1
    fi
}

validate_sources() {
    if [[ ! -f "$DESKTOP_SOURCE" ]]; then
        echo "No se encontró $DESKTOP_SOURCE" >&2
        exit 1
    fi
    if [[ ! -f "$ICON_SOURCE" ]]; then
        echo "No se encontró $ICON_SOURCE" >&2
        exit 1
    fi
}

build_release() {
    require_command cargo
    echo "Compilando Fotos en modo release..."
    CARGO_TARGET_DIR="$PROJECT_DIR/target" cargo build \
        --manifest-path "$PROJECT_DIR/Cargo.toml" \
        --package visor_fotos \
        --release \
        --locked
}

install_application() {
    require_command install
    require_command sed

    if [[ ! -x "$RELEASE_BIN" ]]; then
        echo "No existe el binario release: $RELEASE_BIN" >&2
        echo "Ejecutá primero: ./install.sh --full" >&2
        exit 1
    fi

    echo "Instalando binario..."
    install -Dm755 "$RELEASE_BIN" "$INSTALLED_BIN"

    echo "Instalando acceso de aplicación..."
    sed "s|^Exec=.*|Exec=$INSTALLED_BIN %F|" "$DESKTOP_SOURCE" > "$TMP_DIR/$APP_ID.desktop"
    install -Dm644 "$TMP_DIR/$APP_ID.desktop" "$INSTALLED_DESKTOP"

    if command -v desktop-file-validate >/dev/null 2>&1; then
        desktop-file-validate "$INSTALLED_DESKTOP"
    fi
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$APPLICATIONS_DIR"
    fi
}

install_icons() {
    require_command install
    require_command file

    echo "Imagen de origen: $ICON_SOURCE"

    # Determinar si tenemos ImageMagick instalado
    if command -v magick >/dev/null 2>&1; then
        RESIZE_CMD="magick"
    elif command -v convert >/dev/null 2>&1; then
        RESIZE_CMD="convert"
    else
        echo "Aviso: no se encontró ImageMagick (magick o convert). Instalando icono original sin redimensionar."
        RESIZE_CMD=""
    fi

    if [[ -n "$RESIZE_CMD" ]]; then
        echo "Generando e instalando iconos..."
        for size in 32 48 64 128 256 512; do
            resized_icon="$TMP_DIR/$APP_ID-$size.png"
            $RESIZE_CMD "$ICON_SOURCE" -resize "${size}x${size}" "$resized_icon"

            install -Dm644 \
                "$resized_icon" \
                "$ICON_ROOT/${size}x${size}/apps/$APP_ID.png"
        done
    else
        # Fallback si no hay herramientas de redimensionado: instalar en la carpeta genérica/scalable
        install -Dm644 "$ICON_SOURCE" "$ICON_ROOT/scalable/apps/$APP_ID.png"
    fi

    # Forzar actualización en el DE tocando la carpeta (sin depender de gtk-update-icon-cache)
    touch "$ICON_ROOT" 2>/dev/null || true
}

mode="${1:---full}"

case "$mode" in
    --full|-f)
        if (( $# > 1 )); then
            usage >&2
            exit 2
        fi
        action="full"
        ;;
    --copy|-c)
        if (( $# > 1 )); then
            usage >&2
            exit 2
        fi
        action="copy"
        ;;
    --icons|-i)
        if (( $# > 2 )); then
            usage >&2
            exit 2
        fi
        action="icons"
        ICON_SOURCE="${2:-$DEFAULT_ICON_SOURCE}"
        ;;
    --help|-h)
        if (( $# > 1 )); then
            usage >&2
            exit 2
        fi
        usage
        exit 0
        ;;
    *)
        echo "Opción desconocida: $mode" >&2
        usage >&2
        exit 2
        ;;
esac

validate_sources
require_command mktemp
TMP_DIR="$(mktemp -d)"
trap 'rm -rf -- "$TMP_DIR"' EXIT

case "$action" in
    full)
        build_release
        install_application
        install_icons
        ;;
    copy)
        install_application
        install_icons
        ;;
    icons)
        install_icons
        ;;
esac

echo
case "$action" in
    full)
        echo "Instalación completa finalizada."
        ;;
    copy)
        echo "Archivos instalados sin recompilar."
        ;;
    icons)
        echo "Iconos actualizados."
        ;;
esac

if [[ "$action" != "icons" ]]; then
    echo "Binario: $INSTALLED_BIN"
    echo "Acceso:  $INSTALLED_DESKTOP"
fi
echo "Si COSMIC conserva datos anteriores, cerrá sesión y volvé a entrar."
