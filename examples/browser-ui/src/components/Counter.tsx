import { useState } from "react";

/**
 * FEAT-UI-001
 * Interactive counter component that demonstrates traced frontend feature.
 *
 * This component shows a simple but complete user flow:
 * - Display current count state
 * - Handle user click to increment
 * - Update UI reactively
 */
export function Counter() {
  const [count, setCount] = useState(0);

  return (
    <div className="counter">
      <p data-testid="count-display">Count: {count}</p>
      <button
        data-testid="increment-button"
        onClick={() => setCount((currentCount) => currentCount + 1)}
      >
        Increment
      </button>
    </div>
  );
}
