from rest_framework import permissions


class IsStaffOrReadOnly(permissions.BasePermission):
    def has_permission(self, request, view):
        if request.method in permissions.SAFE_METHODS:
            return True

        return request.user.is_staff


class IsTeacherOfSubject(permissions.BasePermission):
    def has_permission(self, request, view):
        print("has_permission---------------")
        print(request.data)
        print(view)
        return True

    def has_object_permission(self, request, view, obj):
        print("has_object_permission---------------")
        print(request.data)
        print(view)
        print(obj)
        return True
