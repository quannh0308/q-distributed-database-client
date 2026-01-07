# FOUNDATION Directory

## Purpose

This directory contains the **complete, original specification documents** for the Q-Distributed-Database Client SDK. These files serve as the authoritative source of truth for the entire project.

## Token Optimization Strategy

To reduce LLM token usage during implementation, we use a two-tier documentation approach:

### FOUNDATION Files (This Directory)
- **Complete specifications** - Full requirements, design, and tasks
- **Reference material** - Consulted when detailed context is needed
- **Rarely modified** - Stable foundation documents
- **High token count** - ~50K+ tokens total

### Working Files (Parent Directory)
- **Minimal context** - Only what's needed for current task
- **Frequently updated** - Changes as tasks progress
- **Low token count** - ~2-3K tokens total
- **Smart references** - Points back to FOUNDATION for details

## Files in This Directory

### requirements.md
Complete requirements document with:
- 14 detailed requirements with EARS-compliant acceptance criteria
- Technical specifications from server codebase
- Network protocol details
- Authentication structures
- Configuration defaults

### design.md
Complete design document with:
- Comprehensive architecture
- Detailed component interfaces
- 42 correctness properties for property-based testing
- Message protocol specifications
- Error handling strategy
- Performance and security considerations

### tasks.md
Complete implementation plan with:
- 18 top-level tasks
- 70+ sub-tasks with detailed descriptions
- Property-based test specifications
- Requirement traceability
- Checkpoint tasks

## How to Use

### For Implementation
1. **Start with working files** in parent directory for current task context
2. **Refer to FOUNDATION** when you need:
   - Complete requirement details
   - Full design specifications
   - All correctness properties
   - Complete task breakdown

### For Updates
- **FOUNDATION files**: Update only when requirements/design fundamentally change
- **Working files**: Update frequently as tasks progress

### For Context Management
- **Kiro reads**: Working files by default (low token usage)
- **Deep dive**: FOUNDATION files when explicitly needed
- **Best practice**: Reference FOUNDATION path in working files

## Benefits

✅ **Reduced Token Usage**: 90%+ reduction in typical task context
✅ **Faster Responses**: Less context to process
✅ **Focused Work**: Only see what's relevant to current task
✅ **Complete Reference**: Full specs always available when needed
✅ **Better Continuity**: Clear progression through tasks

## Example Workflow

```
1. Read tasks.md (working file) → See current task
2. Implement current task → Focus on specific requirements
3. Need design details? → Check FOUNDATION/design.md
4. Complete task → Update tasks.md progress
5. Move to next task → tasks.md shows next context
```

## Maintenance

- **Weekly**: Review working files for accuracy
- **Monthly**: Sync FOUNDATION with any major changes
- **Per Task**: Update working files with current context
- **As Needed**: Consult FOUNDATION for detailed information

---

This structure is inspired by the q-distributed-database server repository's successful token optimization strategy.
