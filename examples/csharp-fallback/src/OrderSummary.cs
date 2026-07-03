namespace Example.CsharpFallback;

public sealed class OrderSummary
{
    public string Render(int openOrders)
    {
        return $"open orders: {openOrders}";
    }
}
