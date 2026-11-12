import type { Metadata } from "next";
import { AuthForm } from "@/components/AuthForm";
import { login } from "@/app/actions/auth";

export const metadata: Metadata = { title: "Log in" };

export default function LoginPage() {
  return (
    <div className="mx-auto grid max-w-sm gap-6 pt-8">
      <h1 className="text-3xl font-extrabold tracking-tight">Log in</h1>
      <AuthForm mode="login" action={login} />
    </div>
  );
}
