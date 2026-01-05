#!/usr/bin/env python3
import json
import requests
import sys

def format_time(time_str):
    return time_str.zfill(4)

def format_temp(temp):
    return f"{temp}°C"

def format_chances(hour):
    chances = {
        "chanceofrain": "Rain",
        "chanceofsnow": "Snow",
        "chanceofthunder": "Storm",
        "chanceofwindy": "Wind"
    }
    
    conditions = []
    for event, name in chances.items():
        if int(hour[event]) > 0:
            conditions.append(f"{name} {hour[event]}%")
    return ", ".join(conditions)

try:
    weather = requests.get("https://wttr.in/?format=j1").json()
except:
    print(json.dumps({"text": "Unavailable", "tooltip": "Weather service unavailable"}))
    sys.exit()

current = weather['current_condition'][0]
today = weather['weather'][0]
temp = current['feelsLikeC']
desc = current['weatherDesc'][0]['value']
icon_map = {
    "Sunny": "",
    "Clear": "",
    "Partly cloudy": "",
    "Cloudy": "",
    "Overcast": "",
    "Mist": "",
    "Fog": "",
    "Rain": "",
    "Heavy rain": "",
    "Light rain": "",
    "Patchy rain possible": "",
    "Snow": "",
    "Light snow": "",
    "Heavy snow": "",
    "Thunder": "",
    "Thundery outbreaks possible": ""
}
icon = icon_map.get(desc, "")

tooltip = f"<b>{desc}</b>\n"
tooltip += f"Feels like: {temp}°C\n"
tooltip += f"Wind: {current['windspeedKmph']}km/h\n"
tooltip += f"Humidity: {current['humidity']}%\n"

print(json.dumps({
    "text": f"{icon} {temp}°C",
    "tooltip": tooltip,
    "class": "weather"
}))
