#[cfg(test)]
mod tests {
    use crate::metrics::{Counter, Gauge, Histogram, MetricsRegistry};

    // Counter tests
    #[test]
    fn test_counter_new() {
        let counter = Counter::new();
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn test_counter_inc() {
        let counter = Counter::new();
        counter.inc();
        assert_eq!(counter.get(), 1);
    }

    #[test]
    fn test_counter_add() {
        let counter = Counter::new();
        counter.add(5);
        assert_eq!(counter.get(), 5);
        counter.add(10);
        assert_eq!(counter.get(), 15);
    }

    #[test]
    fn test_counter_inc_by() {
        let counter = Counter::new();
        counter.inc_by(3);
        assert_eq!(counter.get(), 3);
        counter.inc_by(7);
        assert_eq!(counter.get(), 10);
    }

    #[test]
    fn test_counter_reset() {
        let counter = Counter::new();
        counter.add(100);
        assert_eq!(counter.get(), 100);
        counter.reset();
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn test_counter_clone() {
        let counter1 = Counter::new();
        counter1.add(42);

        let counter2 = counter1.clone();
        assert_eq!(counter2.get(), 42);

        // Both should share the same underlying value
        counter1.inc();
        assert_eq!(counter1.get(), 43);
        assert_eq!(counter2.get(), 43);
    }

    // Gauge tests
    #[test]
    fn test_gauge_new() {
        let gauge = Gauge::new();
        assert_eq!(gauge.get(), 0);
    }

    #[test]
    fn test_gauge_set() {
        let gauge = Gauge::new();
        gauge.set(10);
        assert_eq!(gauge.get(), 10);
        gauge.set(-5);
        assert_eq!(gauge.get(), -5);
    }

    #[test]
    fn test_gauge_inc() {
        let gauge = Gauge::new();
        gauge.inc();
        assert_eq!(gauge.get(), 1);
        gauge.inc();
        assert_eq!(gauge.get(), 2);
    }

    #[test]
    fn test_gauge_inc_by() {
        let gauge = Gauge::new();
        gauge.inc_by(5);
        assert_eq!(gauge.get(), 5);
        gauge.inc_by(10);
        assert_eq!(gauge.get(), 15);
    }

    #[test]
    fn test_gauge_dec() {
        let gauge = Gauge::new();
        gauge.set(10);
        gauge.dec();
        assert_eq!(gauge.get(), 9);
        gauge.dec();
        assert_eq!(gauge.get(), 8);
    }

    #[test]
    fn test_gauge_dec_by() {
        let gauge = Gauge::new();
        gauge.set(20);
        gauge.dec_by(5);
        assert_eq!(gauge.get(), 15);
        gauge.dec_by(10);
        assert_eq!(gauge.get(), 5);
    }

    #[test]
    fn test_gauge_reset() {
        let gauge = Gauge::new();
        gauge.set(100);
        assert_eq!(gauge.get(), 100);
        gauge.reset();
        assert_eq!(gauge.get(), 0);
    }

    #[test]
    fn test_gauge_clone() {
        let gauge1 = Gauge::new();
        gauge1.set(42);

        let gauge2 = gauge1.clone();
        assert_eq!(gauge2.get(), 42);

        // Both should share the same underlying value
        gauge1.inc();
        assert_eq!(gauge1.get(), 43);
        assert_eq!(gauge2.get(), 43);
    }

    // Histogram tests
    #[test]
    fn test_histogram_new() {
        let hist = Histogram::new(vec![10.0, 50.0, 100.0]);
        assert_eq!(hist.count(), 0);
        assert_eq!(hist.sum(), 0.0);
    }

    #[test]
    fn test_histogram_observe() {
        let hist = Histogram::new(vec![10.0, 50.0, 100.0]);

        hist.observe(5.0);
        hist.observe(25.0);
        hist.observe(75.0);

        assert_eq!(hist.count(), 3);
        assert!((hist.sum() - 105.0).abs() < 0.001);
    }

    #[test]
    fn test_histogram_mean() {
        let hist = Histogram::new(vec![10.0, 50.0, 100.0]);

        hist.observe(10.0);
        hist.observe(20.0);
        hist.observe(30.0);

        assert_eq!(hist.count(), 3);
        assert!((hist.mean() - 20.0).abs() < 0.001);
    }

    #[test]
    fn test_histogram_mean_zero_count() {
        let hist = Histogram::new(vec![10.0, 50.0, 100.0]);
        assert_eq!(hist.mean(), 0.0);
    }

    #[test]
    fn test_histogram_buckets() {
        let hist = Histogram::new(vec![10.0, 50.0, 100.0]);

        hist.observe(5.0); // bucket 0 (<=10)
        hist.observe(15.0); // bucket 1 (<=50)
        hist.observe(60.0); // bucket 2 (<=100)
        hist.observe(150.0); // bucket 3 (>100)

        let buckets = hist.buckets();
        assert_eq!(buckets.len(), 3);
        assert_eq!(buckets[0], (10.0, 1));
        assert_eq!(buckets[1], (50.0, 1));
        assert_eq!(buckets[2], (100.0, 1));
    }

    #[test]
    fn test_histogram_cumulative_buckets() {
        let hist = Histogram::new(vec![10.0, 50.0, 100.0]);

        hist.observe(5.0);
        hist.observe(15.0);
        hist.observe(60.0);
        hist.observe(150.0);

        let cumulative = hist.cumulative_buckets();
        assert_eq!(cumulative.len(), 3);
        assert_eq!(cumulative[0], (10.0, 1)); // 1 value <= 10
        assert_eq!(cumulative[1], (50.0, 2)); // 2 values <= 50
        assert_eq!(cumulative[2], (100.0, 3)); // 3 values <= 100
    }

    #[test]
    fn test_histogram_snapshot() {
        let hist = Histogram::new(vec![10.0, 50.0, 100.0]);

        hist.observe(5.0);
        hist.observe(25.0);
        hist.observe(75.0);

        let snapshot = hist.snapshot();
        assert_eq!(snapshot.count, 3);
        assert!((snapshot.sum - 105.0).abs() < 0.001);
        assert_eq!(snapshot.buckets.len(), 3);
    }

    #[test]
    fn test_histogram_reset() {
        let hist = Histogram::new(vec![10.0, 50.0, 100.0]);

        hist.observe(5.0);
        hist.observe(25.0);

        assert_eq!(hist.count(), 2);

        hist.reset();

        assert_eq!(hist.count(), 0);
        assert_eq!(hist.sum(), 0.0);
    }

    #[test]
    fn test_histogram_clone() {
        let hist1 = Histogram::new(vec![10.0, 50.0, 100.0]);
        hist1.observe(25.0);

        let hist2 = hist1.clone();
        assert_eq!(hist2.count(), 1);

        // Both should share the same underlying data
        hist1.observe(50.0);
        assert_eq!(hist1.count(), 2);
        assert_eq!(hist2.count(), 2);
    }

    // MetricsRegistry tests
    #[test]
    fn test_registry_new() {
        let registry = MetricsRegistry::new();
        assert!(registry.counter_names().is_empty());
        assert!(registry.gauge_names().is_empty());
        assert!(registry.histogram_names().is_empty());
    }

    #[test]
    fn test_registry_counter() {
        let registry = MetricsRegistry::new();
        let counter = registry.counter("test.counter");

        counter.add(5);
        assert_eq!(counter.get(), 5);

        // Getting the same counter should return the same instance
        let counter2 = registry.counter("test.counter");
        assert_eq!(counter2.get(), 5);
    }

    #[test]
    fn test_registry_gauge() {
        let registry = MetricsRegistry::new();
        let gauge = registry.gauge("test.gauge");

        gauge.set(10);
        assert_eq!(gauge.get(), 10);

        // Getting the same gauge should return the same instance
        let gauge2 = registry.gauge("test.gauge");
        assert_eq!(gauge2.get(), 10);
    }

    #[test]
    fn test_registry_histogram() {
        let registry = MetricsRegistry::new();
        let hist = registry.histogram("test.latency", vec![10.0, 50.0, 100.0]);

        hist.observe(25.0);
        assert_eq!(hist.count(), 1);

        // Getting the same histogram should return the same instance
        let hist2 = registry.histogram("test.latency", vec![1.0, 2.0]); // Different buckets but same name
        assert_eq!(hist2.count(), 1); // Should be the same histogram, not a new one
    }

    #[test]
    fn test_registry_counter_names() {
        let registry = MetricsRegistry::new();
        registry.counter("foo");
        registry.counter("bar");
        registry.counter("baz");

        let names = registry.counter_names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"foo".to_string()));
        assert!(names.contains(&"bar".to_string()));
        assert!(names.contains(&"baz".to_string()));
    }

    #[test]
    fn test_registry_gauge_names() {
        let registry = MetricsRegistry::new();
        registry.gauge("cpu");
        registry.gauge("memory");

        let names = registry.gauge_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"cpu".to_string()));
        assert!(names.contains(&"memory".to_string()));
    }

    #[test]
    fn test_registry_histogram_names() {
        let registry = MetricsRegistry::new();
        registry.histogram("latency", vec![10.0, 50.0]);
        registry.histogram("size", vec![100.0, 1000.0]);

        let names = registry.histogram_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"latency".to_string()));
        assert!(names.contains(&"size".to_string()));
    }

    #[test]
    fn test_registry_clear() {
        let registry = MetricsRegistry::new();
        registry.counter("test.counter");
        registry.gauge("test.gauge");
        registry.histogram("test.hist", vec![10.0]);

        assert_eq!(registry.counter_names().len(), 1);
        assert_eq!(registry.gauge_names().len(), 1);
        assert_eq!(registry.histogram_names().len(), 1);

        registry.clear();

        assert!(registry.counter_names().is_empty());
        assert!(registry.gauge_names().is_empty());
        assert!(registry.histogram_names().is_empty());
    }

    #[test]
    fn test_registry_multiple_metrics() {
        let registry = MetricsRegistry::new();

        // Create multiple different metrics
        registry.counter("requests.total");
        registry.counter("errors.total");
        registry.gauge("cpu.usage");
        registry.gauge("memory.usage");
        registry.histogram("request.latency", vec![10.0, 50.0, 100.0, 500.0]);
        registry.histogram("response.size", vec![1024.0, 10240.0, 102400.0]);

        assert_eq!(registry.counter_names().len(), 2);
        assert_eq!(registry.gauge_names().len(), 2);
        assert_eq!(registry.histogram_names().len(), 2);
    }

    #[tokio::test]
    async fn test_registry_async_compatibility() {
        let registry = MetricsRegistry::new();

        // Simulate async increment
        let counter = registry.counter("async.counter");
        counter.add(1);

        let gauge = registry.gauge("async.gauge");
        gauge.set(100);

        let hist = registry.histogram("async.latency", vec![10.0, 50.0, 100.0]);
        hist.observe(25.0);

        assert_eq!(counter.get(), 1);
        assert_eq!(gauge.get(), 100);
        assert_eq!(hist.count(), 1);
    }
}
