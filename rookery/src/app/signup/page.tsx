import type { Metadata } from "next";
import { AuthForm } from "@/components/AuthForm";
import { signup } from "@/app/actions/auth";

export const metadata: Metadata = { title: "Create account" };

export default function SignupPage() {
  return (
    <div className="mx-auto grid max-w-sm gap-6 pt-8">
      <h1 className="text-3xl font-extrabold tracking-tight">Create your account</h1>
      <AuthForm mode="signup" action={signup} />
    </div>
  );
}
