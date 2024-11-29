// data stores for reactive ui
document.addEventListener('alpine:init', () => {
    Alpine.store('selectedVehicle', {
        displayText: "transitmap.io",
        stops: [],
        delays: [],
        currentDelay: 0,
        stopSequence: 0,
        agencyName: "",
        
        update(vehicle) {
            this.displayText = vehicle.getDisplayText();
            this.stops = vehicle.stops ? vehicle.stops : [];
            this.delays = vehicle.delays ? vehicle.delays : [];
            this.agencyName = vehicle.agency_name ? vehicle.agency_name : "";
            this.currentDelay = vehicle.delay ? vehicle.delay : 0;
            this.stopSequence = vehicle.stop_seq ? vehicle.stop_seq : 0;
        },

        getFormattedDelay(stopSequence) {
            let delay = this.delays[stopSequence - 1];
            if (!delay) {
                return ""
            }
            let res = ""
            if (delay < 0) {
                res += '<span style="color: green"> - '
            } else {
                res += '<span style="color: red"> + '
            }
            if (delay >= 60) {
                res += Math.floor(Math.abs(delay) / 60) + "m";
            }
            if (delay % 60 != 0) {
                res += Math.abs(delay) % 60 + "s";
            }
            return res + "</span>";
        }
    })
})
