package example.app;

import static org.junit.jupiter.api.Assertions.assertFalse;

import org.junit.jupiter.api.Test;

class OrderSummaryTest {
    @Test
    void JavaRequirementTest() {
        assertFalse(new OrderSummary().JavaFeatureImpl().isEmpty());
    }
}
