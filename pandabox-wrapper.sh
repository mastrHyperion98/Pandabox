#!/bin/bash
# Wrapper script to ensure proper clipboard access in Flatpak on both X11 and Wayland

# Detect and configure the display server
if [ -n "$WAYLAND_DISPLAY" ]; then
    echo "Running on Wayland"
    # Ensure Wayland socket is accessible
    export WAYLAND_DISPLAY="${WAYLAND_DISPLAY}"
elif [ -n "$DISPLAY" ]; then
    echo "Running on X11"
    # Ensure X11 display is set
    export DISPLAY="${DISPLAY}"
else
    echo "Warning: No display server detected, attempting auto-detection"
fi

# Run the actual application
exec /app/bin/Pandabox "$@"
