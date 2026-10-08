# Gameplay systems

## Car movement

The car uses an intentionally arcade-like model:

1. Holding accelerate increases speed toward `MAX_SPEED`.
2. Holding brake decreases speed toward a limited reverse speed.
3. Releasing both pedals moves speed toward zero using a deceleration rate.
4. Steering rotates around the Y axis. The turn amount scales with the absolute speed, so the car is easier to control while nearly stopped.
5. The car moves along its local forward vector each frame.

The model has no rigid-body physics, suspension, tire friction, or collision meshes. This keeps the first milestone readable and leaves room for a future physics layer.

## Track constraint

The road is represented as a rectangular ring. A position is valid when it is inside the outer rectangle and outside the inner rectangle. If a movement step produces an invalid position, the car returns to its previous position and its speed is reflected and reduced. This prevents the car from escaping the play area without requiring a physics engine.

## Countdown and timer

The countdown resource advances once per second through `3`, `2`, `1`, and `GO!`. Movement only runs in `GameState::Racing`, so the car cannot move during the countdown. The race timer starts on entry to `Racing`, increments with frame time, and is stopped when the finish gate is crossed.

## Finish detection

The player starts to the left of the visible gate and drives right along the bottom straight. `RaceProgress` becomes armed after the player reaches the far side of that straight. The race then finishes when the car crosses the gate from negative X to positive X while moving forward and staying on the road. Arming prevents the initial start position from immediately counting as a completed lap.

This is a minimal one-lap trigger. A future checkpoint system can replace `armed` with an ordered list of sectors and use the same `RaceState` transition.
