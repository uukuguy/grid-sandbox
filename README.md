# Fix for Django QuerySet.exists() Ignoring using() Database Router

## Overview

This repository contains fixes and workarounds for a Django bug where `QuerySet.exists()` ignores the database specified by `.using()`, causing queries to be executed on the default database instead.

## The Problem

```python
# This should query 'other_db', but may query 'default' instead
MyModel.objects.using('other_db').filter(name='test').exists()
```

## Files Included

1. **BUG_EXPLANATION.md** - Detailed explanation of the bug and its root cause
2. **django_exists_using_fix.patch** - Patch file for Django source code
3. **monkey_patch_exists_fix.py** - Monkey patch solution for immediate use
4. **apps.py** - Example Django AppConfig with integrated fix
5. **test_exists_using.py** - Test cases to verify the bug and fix

## Quick Fix Options

### Option 1: Monkey Patch (Recommended for Immediate Fix)

Add this to your app's `apps.py` in the `ready()` method:

```python
from django.apps import AppConfig
from django.db.models.query import QuerySet

class YourAppConfig(AppConfig):
    name = 'your_app'
    
    def ready(self):
        original_exists = QuerySet.exists
        
        def fixed_exists(self):
            if self.query.combinator:
                raise NotImplementedError()
            db = self._db if self._db is not None else self.db
            if self._result_cache is None:
                return self.query.has_results(using=db)
            return bool(self._result_cache)
        
        QuerySet.exists = fixed_exists
```

### Option 2: Use count() Instead (Temporary Workaround)

```python
# Instead of:
MyModel.objects.using('other_db').filter(name='test').exists()

# Use:
MyModel.objects.using('other_db').filter(name='test').count() > 0
```

### Option 3: Apply Django Patch

Apply the patch from `django_exists_using_fix.patch` to your Django installation:

```bash
patch -p1 < django_exists_using_fix.patch
```

## Testing

Run the test cases to verify the fix:

```bash
python manage.py test test_exists_using
```

Or create a test in your test suite:

```python
from django.test import TestCase

class ExistsUsingTest(TestCase):
    databases = {'default', 'other_db'}
    
    def test_exists_respects_using(self):
        # Create object in default database
        MyModel.objects.create(name='test')
        
        # Check in other_db where object doesn't exist
        result = MyModel.objects.using('other_db').filter(name='test').exists()
        
        # Should be False since object is only in default
        self.assertFalse(result)
```

## How It Works

The issue occurs because `QuerySet.exists()` uses `self.db` property, which may not properly respect the `_db` attribute set by the `using()` method.

The fix ensures that:
1. We check `self._db` first (set by `using()`)
2. Fall back to `self.db` if `_db` is not set
3. Pass the correct database to `query.has_results()`

## Affected Django Versions

This issue affects multiple Django versions. Check your specific version's implementation of `QuerySet.exists()` in `django/db/models/query.py`.

## Related Django Documentation

- [Multi-Database Support](https://docs.djangoproject.com/en/stable/topics/db/multi-db/)
- [QuerySet API Reference](https://docs.djangoproject.com/en/stable/ref/models/querysets/)

## Contributing

If you find additional issues or have improvements, please:
1. Fork the repository
2. Create a test case demonstrating the issue
3. Submit a pull request

## License

This fix is provided as-is to help resolve the Django bug. Django itself is licensed under the BSD license.
