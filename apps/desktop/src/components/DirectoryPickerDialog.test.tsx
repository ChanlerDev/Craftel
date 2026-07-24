import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";
import { fakeApi } from "../test/fake";
import { DirectoryPickerDialog } from "./DirectoryPickerDialog";

const root = { path: "/Users/dev", parent: "/Users", entries: [
  { name: ".config", path: "/Users/dev/.config", hidden: true },
  { name: "work", path: "/Users/dev/work", hidden: false },
] };
const opener = { current: null };

test("single click selects a folder while double click enters it", async () => {
  const nested = { path: "/Users/dev/work", parent: "/Users/dev", entries: [] };
  const listDirectory = vi.fn().mockResolvedValueOnce(root).mockResolvedValueOnce(nested);
  const add = vi.fn().mockResolvedValue(true);
  render(<DirectoryPickerDialog api={fakeApi({ listDirectory })} opener={opener} onCancel={vi.fn()} onAdd={add} />);
  const work = await screen.findByRole("button", { name: "work" });
  await userEvent.click(work);
  expect(work).toHaveAttribute("aria-pressed", "true");
  await userEvent.dblClick(work);
  expect(await screen.findByDisplayValue("/Users/dev/work")).toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "Add Project" }));
  expect(add).toHaveBeenCalledWith("/Users/dev/work");
});

test("supports typed navigation, parent navigation, errors, and keyboard entry", async () => {
  const parent = { path: "/Users", parent: "/", entries: [] };
  const listDirectory = vi.fn(async (path?: string) => {
    if (path === "/bad") throw { code: "invalid_path", message: "That folder does not exist" };
    if (path === "/Users") return parent;
    if (path === "/Users/dev/work") return { path, parent: "/Users/dev", entries: [] };
    return root;
  });
  render(<DirectoryPickerDialog api={fakeApi({ listDirectory })} opener={opener} onCancel={vi.fn()} onAdd={vi.fn()} />);
  const input = await screen.findByLabelText("Current folder path");
  await userEvent.clear(input);
  await userEvent.type(input, "/bad{Enter}");
  expect(await screen.findByRole("alert")).toHaveTextContent("That folder does not exist");
  expect(input).toHaveValue("/bad");
  expect(screen.getByRole("button", { name: "Add Project" })).toBeDisabled();
  await userEvent.clear(input);
  await userEvent.type(input, "/Users/dev{Enter}");
  const work = await screen.findByRole("button", { name: "work" });
  work.focus();
  await userEvent.keyboard("{ArrowRight}");
  expect(await screen.findByDisplayValue("/Users/dev/work")).toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "Go to parent folder" }));
  await waitFor(() => expect(input).toHaveValue("/Users/dev"));
});

test("shows loading and empty states, adds the current folder by default, and cancels with Escape", async () => {
  const add = vi.fn().mockResolvedValue(true), cancel = vi.fn();
  render(<DirectoryPickerDialog api={fakeApi({ listDirectory: vi.fn().mockResolvedValue({path:"/",parent:null,entries:[]}) })} opener={opener} onCancel={cancel} onAdd={add} />);
  expect(screen.getByRole("status")).toHaveTextContent("Loading folders");
  expect(await screen.findByText("This folder has no subfolders.")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Go to parent folder" })).toBeDisabled();
  await userEvent.keyboard("{Escape}");
  expect(cancel).toHaveBeenCalledOnce();
  await userEvent.click(screen.getByRole("button", { name: "Add Project" }));
  expect(add).toHaveBeenCalledWith("/");
});

test("keeps registration failures visible and allows retry", async () => {
  const add = vi.fn().mockRejectedValueOnce({ message: "Project is already registered" }).mockResolvedValueOnce(true);
  render(<DirectoryPickerDialog api={fakeApi({ listDirectory: vi.fn().mockResolvedValue(root) })} opener={opener} onCancel={vi.fn()} onAdd={add} />);
  await screen.findByRole("button", { name: "work" });
  await userEvent.click(screen.getByRole("button", { name: "Add Project" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("already registered");
  expect(screen.getByRole("dialog")).toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "Add Project" }));
  expect(add).toHaveBeenCalledTimes(2);
});
