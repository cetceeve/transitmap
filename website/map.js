{ // we use JS scoping, to make sure nothing leaks into global scope
    function initiateLeaflet() {
        let map = L.map('map', {
            center:  [59.34563446044922, 18.071327209472656],
            zoom: 16,
            renderer: L.canvas(),
            zoomControl: false,
        });
        L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
            maxZoom: 19,
            attribution: '&copy; <a href="http://www.openstreetmap.org/copyright">OpenStreetMap</a>'
        }).addTo(map);

        return map;
    }

    map = initiateLeaflet();

    function setMapViewMode(mode) {
        mapViewMode = mode;
        console.log(mode);
    }
    L.Control.CustomViewControl = L.Control.extend({
        onAdd: function(map) {
            let template = document.createElement("template");
            template.innerHTML = "<select><option value='delay' onclick='setMapViewMode(\"delay\")'>Delay View</option><option value='basic' onclick='setMapViewMode(\"basic\")'>Basic View</option></select>"
            return template.content.firstChild;
        },
        onRemove: function(map) { }
    });
    L.control.customViewControl = function(opts) {
        return new L.Control.CustomViewControl(opts);
    }
    L.control.customViewControl({ position: "topright" }).addTo(map);

    map.locate({ watch: true, enableHighAccuracy: true });
    map.on("locationfound", function(e) {
        let latlng = [e.latitude, e.longitude];
        if (!userPosition) {
            this.flyTo(latlng, 16);
        }
        userPosition = {
            latlng,
            accuracy: e.accuracy,
        };
    });
    map.on("click", async function(e) {
        let zoom = this.getZoom()
        if (zoom < clickableZoomLevel) {
            selectedVehicle = undefined;
            return
        }
        let closestDist = zoomToPointRadius(zoom) + 2;
        let closestVehicle = undefined;
        vehiclesOnScreen.forEach(function(vehicle, _, _) {
            if (vehicle.containerPoint) {
                let dist = vehicle.containerPoint.distanceTo(e.containerPoint);
                if (dist < closestDist) {
                    closestDist = dist;
                    closestVehicle = vehicle;
                }
            }
        });
        selectedVehicle = closestVehicle;
        if (selectedVehicle) {
            Alpine.store("selectedVehicle").update(selectedVehicle);
        }
        // fetch metadata if it is not there yet
        if (selectedVehicle?.trip_id) {
            let vehicle = selectedVehicle;
            let metadata = await getMetadata(vehicle.trip_id);
            vehicle.updateData(metadata);
            Alpine.store("selectedVehicle").update(selectedVehicle);
        }
    });
}
