> Deliberately break something. Take your bookmark manager and introduce a bug: delete a closing bracket, misspell a variable name, or remove a function call. Then pretend you don't know where the bug is. Read the error message. Can you find the problem from the error alone, before fixing it?

Made an early return of the bookmarks in `app.js`:

```
return; item;
```

Told the agent:

```
When running `guild-portfolio/bookmark-manager/index.html` I just see `undefinedundefinedundefinedundefinedundefined` in the bookmark section
```

It fixed it immediately.
