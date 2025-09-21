import { Gesture } from '@use-gesture/vanilla';
import { info } from '@tauri-apps/plugin-log';

interface SwipeData {
  distance: number;
  velocity: number;
  element: Element;
}

interface SwipeGestureConfig {
  target: Element;
  swipeLeft?: (data: SwipeData) => void;
  swipeRight?: (data: SwipeData) => void;
  tap?: (data: { element: Element }) => void;
  threshold?: number;
  velocity?: number;
  debug?: boolean;
}

interface SwipeGestureInstance {
  id: symbol;
  destroy: () => void;
  update: (newConfig: Partial<SwipeGestureConfig>) => void;
}

export default class SwipeGestureManager {
  private activeGestures: Map<symbol, any> = new Map();

  /**
   * Creates a new swipe gesture for the specified target element
   * @param config - Configuration object for the swipe gesture
   * @returns SwipeGestureInstance - Object with destroy and update methods
   */
  create(config: SwipeGestureConfig): SwipeGestureInstance {
    const {
      target,
      swipeLeft = () => {},
      swipeRight = () => {},
      tap = () => {},
      threshold = 100,
      velocity = 0.3,
      debug: debugMode = false
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
      },

      onDragEnd: ({ swipe, direction, distance, velocity: dragVelocity }) => {
        if (!swipe) return;

        const horizontalDistance = Math.abs(distance[0]);
        const horizontalVelocity = Math.abs(dragVelocity[0]);

        if (horizontalDistance >= threshold || horizontalVelocity >= velocity) {
          const swipeData: SwipeData = { 
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

      onClick: ({ event }) => {        
        // Call the tap handler
        tap({ element: target });
      }
    });

    this.activeGestures.set(gestureId, gesture);

    return {
      id: gestureId,
      destroy: () => this.destroy(gestureId),
      update: (newConfig: Partial<SwipeGestureConfig>) => this.update(gestureId, newConfig)
    };
  }

  /**
   * Destroys a specific gesture by ID
   * @param gestureId - The symbol ID of the gesture to destroy
   */
  private destroy(gestureId: symbol): void {
    const gesture = this.activeGestures.get(gestureId);
    if (gesture) {
      gesture.destroy();
      this.activeGestures.delete(gestureId);
    }
  }

  /**
   * Updates the configuration of an existing gesture
   * @param gestureId - The symbol ID of the gesture to update
   * @param newConfig - Partial configuration to update
   */
  private update(gestureId: symbol, newConfig: Partial<SwipeGestureConfig>): void {
    const gesture = this.activeGestures.get(gestureId);
    if (gesture && newConfig) {
      // Note: @use-gesture/vanilla may not support dynamic config updates
      // For now, we'll log the update attempt
      info(`SwipeGestureManager: Update requested for gesture ${gestureId.toString()}`);
      // To properly update, you would need to destroy and recreate the gesture
    }
  }

  /**
   * Destroys all active gestures
   */
  destroyAll(): void {
    this.activeGestures.forEach(gesture => gesture.destroy());
    this.activeGestures.clear();
  }

  /**
   * Gets the count of active gestures
   * @returns number - Count of active gestures
   */
  getActiveCount(): number {
    return this.activeGestures.size;
  }
}
