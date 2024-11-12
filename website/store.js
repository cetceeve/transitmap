// data stores for reactive ui
document.addEventListener('alpine:init', () => {
    Alpine.store('selectedVehicle', {
        displayText: "transitmap.io",
        stops: [],
        delays: [],
        agencyName: "",
        
        update(vehicle) {
            this.displayText = vehicle.getDisplayText();
            this.stops = vehicle.stops ? vehicle.stops : [];
            this.delays = vehicle.delays ? vehicle.delays : [];
            this.agencyName = vehicle.agency_name ? vehicle.agency_name : "";
        }
    })
})
