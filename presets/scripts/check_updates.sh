#!/bin/bash
if ! command -v checkupdates &> /dev/null; then
    echo '{"text": "N/A", "tooltip": "pacman-contrib not installed"}'
    exit 0
fi

pkg_updates=$(checkupdates 2> /dev/null | wc -l)
aur_updates=0

if command -v yay &> /dev/null; then
    aur_updates=$(yay -Qua 2> /dev/null | wc -l)
elif command -v paru &> /dev/null; then
    aur_updates=$(paru -Qua 2> /dev/null | wc -l)
fi

total=$((pkg_updates + aur_updates))

if [ "$total" -eq 0 ]; then
    echo '{"text": "", "tooltip": "System is up to date"}'
else
    tooltip="<b>Updates Available</b>\nPacman: $pkg_updates\nAUR: $aur_updates"
    echo "{\"text\": \" $total\", \"tooltip\": \"$tooltip\", \"class\": \"updates\"}"
fi
