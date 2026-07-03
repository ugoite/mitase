require "minitest/autorun"
require_relative "../lib/order_summary"

class OrderSummaryTest < Minitest::Test
  def test_ruby_requirement
    refute_empty OrderSummary.new.ruby_feature_impl
  end
end
