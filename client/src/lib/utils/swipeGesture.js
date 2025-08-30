import { Gesture } from '@use-gesture/vanilla';

class SwipeGestureManager {
  constructor() {
    this.activeGestures = new Map();
  }

  create(config = {}) {
    const {
      target,
      swipeLeft = () => {},
      swipeRight = () => {},
      threshold = 100,
      velocity = 0.3,
      debug = false
    } = config;

    if (!target) {
      throw new Error('SwipeGestureManager: target element is required');
    }

    const gestureId = Symbol('swipe-gesture');

    const gesture = new Gesture(target, {
      onDrag: ({ event, direction, distance }) => {
        // Prevent default for horizontal swipes
        if (Math.abs(direction[0]) > Math.abs(direction[1])) {
          event.preventDefault();
        }
        
        if (debug) {
          console.log('SwipeGesture: Dragging', { direction, distance });
        }
      },

      onDragEnd: ({ swipe, direction, distance, velocity: dragVelocity }) => {
        if (!swipe) return;

        const horizontalDistance = Math.abs(distance[0]);
        const horizontalVelocity = Math.abs(dragVelocity[0]);

        if (debug) {
          console.log('SwipeGesture: Detected', {
            direction: direction[0] > 0 ? 'right' : 'left',
            distance: horizontalDistance,
            velocity: horizontalVelocity
          });
        }

        // Check thresholds
        if (horizontalDistance >= threshold || horizontalVelocity >= velocity) {
          const swipeData = { 
            distance: horizontalDistance, 
            velocity: horizontalVelocity,
            element: target
          };

          if (direction[0] > 0) {
            // Left to right swipe
            swipeRight(swipeData);
          } else {
            // Right to left swipe
            swipeLeft(swipeData);
          }
        }
      },

      drag: {
        axis: 'x',
        threshold: 10,
        rubberband: false
      },

      swipe: {
        distance: threshold,
        velocity: velocity
      }
    });

    this.activeGestures.set(gestureId, gesture);

    if (debug) {
      console.log('SwipeGestureManager: Gesture created', { gestureId, target });
    }

    return {
      id: gestureId,
      destroy: () => this.destroy(gestureId),
      update: (newConfig) => this.update(gestureId, newConfig)
    };
  }

  destroy(gestureId) {
    const gesture = this.activeGestures.get(gestureId);
    if (gesture) {
      gesture.destroy();
      this.activeGestures.delete(gestureId);
    }
  }

  update(gestureId, newConfig) {
    const gesture = this.activeGestures.get(gestureId);
    if (gesture && newConfig) {
      const { threshold = 100, velocity = 0.3 } = newConfig;
      gesture.setConfig({
        drag: {
          axis: 'x',
          threshold: 10,
          rubberband: false
        },
        swipe: {
          distance: threshold,
          velocity: velocity
        }
      });
    }
  }

  destroyAll() {
    this.activeGestures.forEach(gesture => gesture.destroy());
    this.activeGestures.clear();
  }

  getActiveCount() {
    return this.activeGestures.size;
  }
}

export const swipeGestureManager = new SwipeGestureManager();
