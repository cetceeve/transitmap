// the leaflet map
let map;

// the current user latlng, available only if user accepts position access
let userPosition;

// hold references to all vehicles
let vehicleHM = new Map();

// holds references to vehicles that should be rendered
let vehiclesOnScreen = new Map();

// holds a reference to the currently selected vehicle
let selectedVehicle;

// the minimum zoom level at which vehicles are clickable
const clickableZoomLevel = 13;

// the view mode of the map
// allowed values: "delay", "basic"
let mapViewMode = "basic";
