from rest_framework import viewsets

from .mixins import AutoUserMixin
from .models import ApplicationRequest
from .serializers import ApplicationRequestSerializer

from backend.permissions import IsStaffOrReadOnly


class ApplicationRequestViewSet(AutoUserMixin, viewsets.ModelViewSet):
    queryset = ApplicationRequest.objects.all()
    serializer_class = ApplicationRequestSerializer
    permission_classes = [IsStaffOrReadOnly]
