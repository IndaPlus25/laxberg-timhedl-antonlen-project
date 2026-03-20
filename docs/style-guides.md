# Style guide

## Code style

We will use the [Rust style guide](https://doc.rust-lang.org/style-guide/) to inform how we structure our code.

We will also follow [Never nesting](https://medium.com/@collinsakuma/never-nesting-bd8c0f2a9ca0).

If a function has a return type and can only fail in one way, **an** option is used. If it can fail multiple ways, **a** Result with a custom error is used.

## Naming convention

A commit should always be prefaced with **an** indicator that **allows** easy **identification** of what type of commit it is. The indicator should be followed by a *":"* and then a short **title** that explains what the commit does. 

| Prefix | Indication |
| - | - |
| *file* | The commit is related to **file management**, i.e creating a new file, moving or deleting |
| *feature* | Adding a new feature to the code base, i.e new function/functionality |
| *bug* | Fixing a bug found in project | 
| *cleanup* | Cleaning up code, removing **fluff** and **formatting** |
| *improve* | Improving **an** already existing function |
| *doc* | For **documentation** of project, i.e specifications or comments |

### Example

*"doc: updating style guide"*

-----------

Our workflow consists of different branches, all these are prefaced with **an** indicator that shows the branch's **depth**. The indicator should be followed by a *"/"* and then a **descriptive** name for the branch. 

| Prefix | Indication |
| - | - |
| *main* | Is the main branch |
| *dev* | Is the working branch for completed features |
| *feature* | Is an offshoot of the **development** branch, for working on features |

### Example

*"feature/blackhole"*