{ // we use JS scoping, to make sure nothing leaks into global scope
	const smoothZoomLevel = 10;

	// smooth animation only if zoomed in
	let animateSmooth = map.getZoom() > smoothZoomLevel;
	let mapIsMoving = false; // animation is paused during map movements

	map.on("movestart", function() {
		mapIsMoving = true;
	})
	map.on("zoomstart", function() {
		mapIsMoving = true;
	})
	map.on("zoomend", function() {
		refreshVehiclesOnScreen()
		mapIsMoving = false;
		animateSmooth = this.getZoom() > smoothZoomLevel;
		canvasLayer.animate();
	})
	// update vehiclesOnScreen when the map moves
	map.on('moveend', function(e) {
		refreshVehiclesOnScreen()
		mapIsMoving = false;
		canvasLayer.animate();
	});

	// vehicle icons
	const busIcon = new Image();
	busIcon.src = "./images/bus_icon.png";
	const trainIcon = new Image();
	trainIcon.src = "./images/train_icon.png";
	const tramIcon = new Image();
	tramIcon.src = "./images/tram_icon.png";
	const metroIcon = new Image();
	metroIcon.src = "./images/metro_icon.png";
	const ferryIcon = new Image();
	ferryIcon.src = "./images/ferry_icon.png";
	const taxiIcon = new Image();
	taxiIcon.src = "./images/taxi_icon.png";
	const otherIcon = new Image();
	otherIcon.src = "./images/other_icon.png";
	const colorsByType = {
		"train": "#FF7600",
		"metro": "#D61355",
		"bus": "#0078FF",
		"tram": "#2BA714",
		"ferry": "#0D1282",
		"taxi": "#FBCB0A",
		"other": "#8B8B8B",
	};
	const iconsByType = {
		"train": trainIcon,
		"metro": metroIcon,
		"bus": busIcon,
		"tram": tramIcon,
		"ferry": ferryIcon,
		"taxi": taxiIcon,
		"other": otherIcon,
	};
	function drawVehicle(ctx, vehicle, zoom, point, pointRadius) {
		let color;
		if (mapViewMode == "delay") {
			if (vehicle.delay) {
				if (vehicle.delay > 120) {
					color = "hsl(0, 100%, 45%)";
				} else if (vehicle.delay > 60) {
					color = "hsl(39, 100%, 50%)";
				} else if (vehicle.delay > -60) {
					color = "hsl(120, 100%, 35%)";
				} else {
					color = "hsl(200, 100%, 40%)";
				}
			} else {
				color = "hsl(120, 100%, 35%)";
			}
		} else {
			// basic view style: colors based on vehicle type
			color = colorsByType[vehicle.type];
		}

		ctx.beginPath();
		ctx.fillStyle = color;
		if (zoom < clickableZoomLevel) {
			ctx.arc(point.x, point.y, pointRadius, 0, 2*Math.PI);
		} else {
			ctx.arc(point.x, point.y, pointRadius, 0, 2*Math.PI);
		}
		ctx.fill();

		// then draw the icon on top
		if (zoom >= clickableZoomLevel) {
			let halfSideLen = pointRadius - pointRadius/3;
			ctx.drawImage(iconsByType[vehicle.type], point.x-halfSideLen, point.y-halfSideLen, 2*halfSideLen, 2*halfSideLen);
		}
	}

	// our custom canvasLayer, used to render vehicles
	const canvasLayer = new L.CustomLayer({
		container: document.createElement("canvas"),
		maxZoom: 19,
	});
	canvasLayer.animate = function() {
		// frame function is called every frame
		let layer = this;
		let frameCounter = 0;
		function frame(timestamp) {
			if (mapIsMoving) {
				return // pause animation when scrolling
			}
			// smooth animation is every 2 frames, slow is every 60 frames
			if (animateSmooth && frameCounter % 2 == 0 || frameCounter % 60 == 0) {
				let canvas = layer.getContainer();
				let ctx = canvas.getContext("2d");
				ctx.clearRect(0, 0, canvas.width, canvas.height);

				// draw vehicles
				let zoom = layer._map.getZoom();
				let pointRadius = zoomToPointRadius(zoom);
				vehiclesOnScreen.forEach(function(vehicle, _, _) {
					let point;
					if (vehicle.animateUntil < timestamp) {
						vehicle.animatedLatlng = null;
						point = layer._map.latLngToContainerPoint(vehicle.animationEndLatlng);
						vehicle.containerPoint = point;
					} else {
						let animationDuration = vehicle.animateUntil - vehicle.animationStart;
						let remainingTime = vehicle.animateUntil - timestamp;
						let percentDone = (animationDuration - remainingTime) / (animationDuration + 1);
						let startPoint = layer._map.latLngToContainerPoint(vehicle.animationStartLatlng);
						let endPoint = layer._map.latLngToContainerPoint(vehicle.animationEndLatlng);
						point = endPoint.multiplyBy(percentDone).add(startPoint.multiplyBy(1 - percentDone));
						vehicle.containerPoint = point;
						if (animateSmooth) {
							vehicle.animatedLatlng = layer._map.containerPointToLatLng(point);
						} else {
							vehicle.animatedLatlng = null;
						}
					}

					// draw
					drawVehicle(ctx, vehicle, zoom, point, pointRadius);
				});
				// highlight selected vehicle
				if (selectedVehicle && vehiclesOnScreen.has(selectedVehicle.id)) {
					// draw the vehicle again on top
					drawVehicle(ctx, selectedVehicle, zoom, selectedVehicle.containerPoint, pointRadius);
					// draw highlight circle
					ctx.beginPath();
					ctx.strokeStyle = "red";
					ctx.lineWidth = 3
					ctx.arc(selectedVehicle.containerPoint.x, selectedVehicle.containerPoint.y, pointRadius+1, 0, 2*Math.PI);
					ctx.stroke();
					// draw textbox
					let displayText = selectedVehicle.getDisplayText();
					let textMetrics = ctx.measureText(displayText);
					let textHeight = textMetrics.actualBoundingBoxAscent + textMetrics.actualBoundingBoxDescent;
					let textWidth = textMetrics.actualBoundingBoxLeft + textMetrics.actualBoundingBoxRight;
					let textLeft = selectedVehicle.containerPoint.x + 3;
					let textBottom = selectedVehicle.containerPoint.y - pointRadius - 7 - textMetrics.actualBoundingBoxDescent;
					let textTop = textBottom - textMetrics.actualBoundingBoxAscent;
					ctx.beginPath();
					ctx.fillStyle = "white";
					ctx.rect(textLeft-2, textTop-2, textWidth+5, textHeight+5);
					ctx.fill();
					ctx.beginPath();
					ctx.strokeStyle = "black";
					ctx.lineWidth = 2
					ctx.rect(textLeft-2, textTop-2, textWidth+5, textHeight+5);
					ctx.stroke();
					ctx.fillStyle = "black";
					ctx.fillText(displayText, textLeft, textBottom);
				}
				// draw user location
				if (userPosition) {
					let point = layer._map.latLngToContainerPoint(userPosition.latlng);
					ctx.fillStyle = "#24b6ff";
					ctx.beginPath();
					ctx.arc(point.x, point.y, 6, 0, 2*Math.PI);
					ctx.fill();
					ctx.beginPath();
					ctx.globalAlpha = 0.3;
					ctx.arc(point.x, point.y, metersToPixels(userPosition.accuracy, map), 0, 2*Math.PI);
					ctx.fill();
					ctx.globalAlpha = 1.0;
				}
			}
			frameCounter++;
			window.requestAnimationFrame(frame);
		}

		// reset bounds and start the animation loop
		let { ctx } = layer.setFullLayerBounds();
		ctx.font = "20px arial"
		frame(performance.now());
	}
	// event handlers for our custom canvasLayer
	canvasLayer.on("layer-mounted", function() {
		this._reset();
		this.animate();
	});

	canvasLayer.addTo(map);
}
