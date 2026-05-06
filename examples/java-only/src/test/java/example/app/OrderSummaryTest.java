package example.app;

import static org.junit.jupiter.api.Assertions.assertFalse;

import org.junit.jupiter.api.Test;

class OrderSummaryTest {
    /** JavaRequirementTest keeps the Java requirement trace readable. */
    @Test
    void JavaRequirementTest() {
        assertFalse(new OrderSummary().JavaFeatureImpl().isEmpty());
    }
}
