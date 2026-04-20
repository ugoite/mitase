import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { Counter } from "./Counter";

/**
 * REQ-UI-001
 * Test that counter increments correctly on user interaction
 */
export function test_counter_increments() {
  describe("Counter", () => {
    it("should increment count when button is clicked", () => {
      render(<Counter />);

      const button = screen.getByTestId("increment-button");
      const display = screen.getByTestId("count-display");

      expect(display.textContent).toBe("Count: 0");

      fireEvent.click(button);
      expect(display.textContent).toBe("Count: 1");

      fireEvent.click(button);
      expect(display.textContent).toBe("Count: 2");
    });
  });
}

test_counter_increments();
