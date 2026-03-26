You are performing a thorough code review. Analyze the code with the rigor of a senior engineer reviewing a pull request.

## Review Target

$ARGUMENTS

## Review Dimensions

Work through each dimension systematically:

### 1. Correctness
- Logic errors, off-by-one, null/undefined handling
- Edge cases and boundary conditions
- Error handling completeness

### 2. Security
- Input validation and sanitization
- Injection vulnerabilities (SQL, command, XSS)
- Sensitive data exposure (secrets, PII in logs)
- Authentication/authorization gaps

### 3. Performance
- Unnecessary allocations or copies
- N+1 queries, missing indexes
- Blocking operations in async context
- Resource leaks (file handles, connections)

### 4. Maintainability
- Naming clarity, code organization
- Dead code, unused imports
- Missing or misleading comments
- Violation of project conventions

### 5. Testing
- Untested code paths
- Missing edge case coverage
- Test quality (assertions, not just execution)

## Output Format

For each finding, use this format:

**[SEVERITY] Category — File:Line**
Description of the issue.
Suggested fix (with code if applicable).

Severity levels: CRITICAL / HIGH / MEDIUM / LOW / INFO

End with a summary: total findings by severity, overall assessment, and top 3 priorities.
